use anyhow::{bail, Error};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tempdir::TempDir;
use tokio::sync::RwLock; // also works from gtk, unlike tokio::fs

use gio::Cancellable;
use glib::WeakRef;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

use crate::thumbnail_generator::ThumbSize;
use podcasts_data::errors::DownloadError;
use podcasts_data::xdg_dirs::CACHED_COVERS_DIR;
use podcasts_data::ShowCoverModel;

// Downloader v3
// if a textures is in the COVER_TEXTURES cache:
//     - Set the image to that texture.
// if file doesn't exist:
//     - Create 0byte placeholder file.
//     - Start Download into tmp file.
//     - Generate thumbnails.
//     - Move the download to the final path.
//     - goto: (if file exists) ↓
// if 0byte exits:
//     - Create FileMonitor
//     - Register load callback on changed
//     - Return the monitor for a widget to keep around until it sets the cover.
//       TODO: drop the monitor after that happens
// if file exists:
//     - check COVER_TEXTURES cache, set image from it if any hits, else
//     - Load the file's texture into the COVER_TEXTURES cache.
//     - set the image to the texture
//     - Only this needs the gtk widget, rest can be done off thread

static CACHE_VALID_DURATION: Lazy<chrono::Duration> = Lazy::new(|| chrono::Duration::weeks(4));

static COVER_TEXTURES: Lazy<RwLock<HashMap<String, gdk::Texture>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

// Create a 0byte file to serve as our lock
async fn create_cover_lock(path: &PathBuf) -> Result<(), Error> {
    info!("Creating 0 byte file at: '{}'", path.display());
    let file = File::create(path)?;
    file.sync_all()?;

    Ok(())
}

fn file_changed(event: gio::FileMonitorEvent, path: PathBuf, image: &WeakRef<gtk::Image>) {
    // info!("FileMonitor changed event: '{}'", event);
    debug!("FileMonitor changed event: '{event:#?}'");

    if event == gio::FileMonitorEvent::MovedIn || event == gio::FileMonitorEvent::Renamed {
        let image = image.clone();
        crate::MAINCONTEXT.spawn_local(async move {
            set_image_from_file_with_tokio(&image, path).await.unwrap();
        });
    }
}

async fn create_file_monitor(
    path: &PathBuf,
    thumb: &PathBuf,
    image: &WeakRef<gtk::Image>,
) -> Result<gio::FileMonitor, Error> {
    let file = gio::File::for_path(path);
    let monitor = file.monitor_file(gio::FileMonitorFlags::WATCH_MOVES, None::<&Cancellable>)?;
    info!("Watching file for renames: '{}'", path.display());
    let thumb = thumb.clone();
    let image = image.clone();
    monitor.connect_changed(move |_, _, _, event| file_changed(event, thumb.clone(), &image));
    Ok(monitor)
}

fn filename_for_download(response: &reqwest::Response) -> &str {
    // Get filename from url if possible
    let ext = response
        .url()
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("tmp-donwload.bin");

    if ext.is_empty() {
        return "tmp-donwload.bin";
    }

    ext
}

pub fn clean_unfinished_downloads() -> Result<(), Error> {
    info!("Starting cover locks cleanup");
    let dir = CACHED_COVERS_DIR.clone();

    for entry in fs::read_dir(dir)? {
        // keep going if any one file fails
        match entry.map(|e| e.path()) {
            Ok(path) => {
                if let Err(err) = cleanup_entry(&path) {
                    error!("failed to cleanup: {} {err}", path.display());
                }
            }
            Err(err) => error!("failed to get path {err}"),
        }
    }

    Ok(())
}

fn cleanup_entry(path: &PathBuf) -> Result<(), Error> {
    if path.is_file() {
        let file = fs::File::open(&path)?;
        let size = file.metadata()?.len();
        if size == 0 {
            drop(file);
            info!("Removing 0byte file: '{}'", path.display());
            fs::remove_file(&path)?;
        } else if path.ends_with(".part") {
            fs::remove_file(&path)?;
        }
    }
    // remove tmp directories of unfinished downloads
    if path.is_dir() {
        if let Some(filename) = path.to_str() {
            if filename.contains("-pdcover.part") {
                info!("Removing unfinished download {filename}");
                // remove_dir_all can be risky if xdg would break,
                // but we are filtering for a "*-pdcover.part*" dir-name
                // and in a "Covers/" subdir, so it should be fine.
                fs::remove_dir_all(&path)?;
            }
        }
    }
    Ok(())
}

/// Returns the URI of a Show's cover directory given it's title.
fn get_cover_file_path(hash: &str) -> PathBuf {
    let mut dir = CACHED_COVERS_DIR.clone();
    // Don't even bother setting a file extension as, we will
    // ultimately end up feeding it into the same loader regardless
    dir.push(hash);
    info!("Constructed cover path: '{}'", dir.display());
    dir
}

pub fn determin_cover_path(pd: &ShowCoverModel, size: Option<ThumbSize>) -> PathBuf {
    // TODO: once we start storing them as strings
    // let hash = u64_to_vec_u8(pd.image_uri_hash().unwrap());
    let hash = if let Some(size) = size {
        format!("{}-{size}", pd.id())
    } else {
        format!("{}", pd.id())
    };
    get_cover_file_path(&hash)
}

fn determin_cover_path_for_update(pd: &ShowCoverModel) -> PathBuf {
    // TODO: once we start storing them as strings
    // let hash = u64_to_vec_u8(pd.image_uri_hash().unwrap());
    let hash = format!("{}-update", pd.id());
    get_cover_file_path(&hash)
}

// FIXME: handle chunked downloads
async fn download_file_to_tmp(
    pd: ShowCoverModel,
    path: PathBuf,
    tmp_path: PathBuf,
) -> Result<(PathBuf, gdk::Texture), DownloadError> {
    // this needs tokio cause of reqwest
    crate::RUNTIME
        .spawn(async move {
            let client = podcasts_data::downloader::client_builder().build()?;
            let response = client.get(pd.image_uri().unwrap()).send().await?;
            //FIXME: check for 200 or redirects, retry for 5xx
            debug!("Status Resp: {}", response.status());

            let filename = filename_for_download(&response);
            let filename = tmp_path.join(filename);
            info!("Downloading file into: '{:?}'", filename);
            let mut dest = tokio::fs::File::create(&filename).await?;

            let mut content = Cursor::new(response.bytes().await?);
            tokio::io::copy(&mut content, &mut dest).await?;

            dest.sync_all().await?;
            drop(dest);

            let texture = tokio_make_texture(&filename, false).await?;

            Ok((filename, texture))
        })
        .await
        .unwrap_or_else(|e| {
            error!("download_file_to_tmp tokio task failed {e}");
            Err(DownloadError::TokioError)
        })
}

async fn download_cover_image(pd: &ShowCoverModel, path: &PathBuf) -> Result<(), DownloadError> {
    let url = pd
        .image_uri()
        .ok_or(DownloadError::NoImageLocation)?
        .to_owned();

    if url.is_empty() {
        return Err(DownloadError::NoImageLocation);
    }

    // Actual download in tokio thread for reqwest.
    // Keep the tmp_dir here, we do the rename later, not in tokio,
    // since we need to generate thumbnails here in the gtk context.
    let tmp_dir = TempDir::new_in(&*CACHED_COVERS_DIR, &format!("{}-pdcover.part", pd.id()))?;
    let (filename, texture) =
        download_file_to_tmp(pd.clone(), path.clone(), tmp_dir.path().into()).await?;

    // Generate thumbnails for the cover
    let _thumbs = crate::thumbnail_generator::generate(&pd, texture)
        .await
        .map_err(|e| {
            error!("Thumbnail generation error {e}");
            DownloadError::ThumbnailError
        })?;
    // TODO could insert requested thumb texture into cache here.

    // Finalize
    // we only rename after thumbnails are generated,
    // so thumbnails can be presumed to exist if the orginal file exists
    fs::rename(&filename, &path)?;
    info!("Cached img into: '{}'", &path.display());
    Ok(())
}

async fn get_cover_file(
    pd: &ShowCoverModel,
    image: &WeakRef<gtk::Image>,
    thumb_size: ThumbSize,
) -> Result<Option<gio::FileMonitor>, Error> {
    let cover = determin_cover_path(&pd, None);
    let thumb = determin_cover_path(&pd, Some(thumb_size));

    // Early return from cache
    let path_string = thumb.to_str().unwrap().to_string();
    let cached_texture = {
        let r = COVER_TEXTURES.read().await;
        r.get(&path_string).cloned()
    };
    if let Some(texture) = cached_texture {
        if let Some(image) = image.upgrade() {
            image.set_paintable(Some(&texture));
        }
        return Ok(None);
    }

    // Lock & start a download
    if !cover.exists() {
        info!(
            "Cover file does not exist, Starting download. {}",
            cover.display()
        );
        create_cover_lock(&cover).await?;
        download_cover_image(&pd, &cover).await?;
    }

    assert!(cover.is_file());
    let file = std::fs::File::open(&cover)?;
    let size = file.metadata()?.len();

    // Assume that a 0 sized file is our lockfile,
    // and any size is a complete cover
    if size == 0 {
        info!("Found zero sized file, creating FileMonitor");
        let monitor = create_file_monitor(&cover, &thumb, image).await?;
        return Ok(Some(monitor));
    } else if !thumb.exists() {
        // Fallback for if we add more/different thumb sizes
        warn!("Cover exists, but thumb is missing, redownloading it!");
        fs::remove_file(&cover)?;
        create_cover_lock(&cover).await?;
        download_cover_image(&pd, &cover).await?;
    } else if !pd.is_cached_image_valid(&CACHE_VALID_DURATION) {
        // Download a potentially updated cover and replace.
        // It won't update all images instantly,
        // but that shouldn't be a big problem.
        let update_path = determin_cover_path_for_update(pd);
        if let Ok(_) = create_cover_lock(&update_path).await {
            download_cover_image(&pd, &update_path).await?;
            fs::rename(&update_path, &cover)?;
        }
    }
    if !thumb.exists() {
        bail!("Failed to generate thumbs");
    }
    info!("Loading cover for '{}'", pd.title());
    set_image_from_file_with_tokio(image, thumb.clone())
        .await
        .unwrap();

    return Ok(None);
}

// TODO maybe instead of an image we could pass a oneshot sender that returns a texture.
// Some early returns would have to check for sender.is_closed() instead of upgrade.
// This change would allow for easy dropping of the FileMonitor and adding support for Pictures.
/// The returned FileMonitor must be stored until the image is set.
pub async fn load_image_async(
    image: &WeakRef<gtk::Image>,
    podcast_id: i32,
    size: ThumbSize,
) -> Option<gio::FileMonitor> {
    use podcasts_data::dbqueries;

    let pd = gio::spawn_blocking(move || dbqueries::get_podcast_cover_from_id(podcast_id).unwrap())
        .await
        .unwrap();
    if let Some(image) = image.upgrade() {
        image.set_tooltip_text(Some(pd.title()));
    } else {
        return None;
    }

    let monitor = get_cover_file(&pd, image, size).await.unwrap();
    monitor
}

async fn set_image_from_file_with_tokio(
    image: &WeakRef<gtk::Image>,
    path: PathBuf,
) -> Result<(), Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    crate::RUNTIME.spawn(async move {
        let texture = tokio_make_texture(&path, true).await?;
        let _ = sender.send(texture);
        Ok::<(), DownloadError>(())
    });

    let image = match image.upgrade() {
        Some(i) => i,
        None => return Ok(()),
    };

    if let Ok(texture) = receiver.await {
        image.set_paintable(Some(&texture));
    }

    Ok(())
}

async fn tokio_make_texture(path: &Path, cache: bool) -> Result<gdk::Texture, DownloadError> {
    let path_string = path.to_str().unwrap().to_string();
    if cache {
        let r = COVER_TEXTURES.read().await;
        if let Some(t) = r.get(&path_string).cloned() {
            return Ok(t);
        };
    }

    match gdk::Texture::from_filename(path) {
        Ok(texture) => {
            if cache {
                let mut w = COVER_TEXTURES.write().await;
                w.insert(path_string, texture.clone());
            }
            Ok(texture)
        }
        Err(err) => {
            error!("Failed to load texture from: {} {err}", path.display());
            Err(DownloadError::FailedToLoadTexture)
        }
    }
}

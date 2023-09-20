use anyhow::Error;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use tempdir::TempDir;
use tokio::sync::oneshot::Sender;
use tokio::sync::RwLock;

use gio::prelude::*;
use gio::Cancellable;
use glib::clone;
use glib::WeakRef;
use gtk::gdk;
use gtk::gio;
use gtk::glib;

use podcasts_data::errors::DownloadError;
use podcasts_data::xdg_dirs::CACHED_COVERS_DIR;
use podcasts_data::xdg_dirs::PODCASTS_CACHE;
use podcasts_data::ShowCoverModel;

// Downloader v2
//  TODO: v3 taking FileMonitor lifetime into account
// determine file path (deterministic)
//
// if file doesn't exist:
//     - Create 0byte placeholder file
//     - Start Download
// if 0byte exits:
//     - Create FileMonitor
//     - Register load callback on changed
// if file exists:
//     - return path
//     - (In the future) Make a paintable cache to avoid creating different texutres
//     - load it
//          - Only this needs the gtk widget, rest can be done off thread
//
// Problem:
// if a download fails and it leaves a 0byte file behind, we have no way of knowing if
// it's still running, or it's state on future application instances
// FIXME: Add the uri in the db, then upon startup/shutdown/download failure, clean up the 0byte file

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

fn file_changed(event: gio::FileMonitorEvent, path: &PathBuf, image: &WeakRef<gtk::Image>) {
    // info!("FileMonitor changed event: '{}'", event);
    debug!("FileMonitor changed event: '{event:#?}'");

    if event == gio::FileMonitorEvent::MovedIn || event == gio::FileMonitorEvent::Renamed {
        let path = path.to_str().unwrap().to_string();
        let image = image.clone();
        crate::MAINCONTEXT.spawn_local(async move {
            set_image_from_file_with_tokio(&image, path).await.unwrap();
        });
    }
}

async fn create_file_monitor(path: &PathBuf, image: &WeakRef<gtk::Image>) -> Result<(), Error> {
    let file = gio::File::for_path(path);
    let monitor = file.monitor_file(gio::FileMonitorFlags::WATCH_MOVES, None::<&Cancellable>)?;
    info!("Watching file for renames: '{}'", path.display());
    let path = path.clone();
    let image = image.clone();
    let handler = monitor.connect_changed(move |_, _, _, event| file_changed(event, &path, &image));
    // FIXME: we ain't getting any more events once it's dropped
    // FIXME: Keep the monitor around for a while. Probably
    // Start returning it upwards and making a tuple/struct of the Monitor/Image|Paintable
    std::mem::forget(monitor);
    std::mem::forget(handler);

    Ok(())
}

fn extract_file_extension(response: &reqwest::Response) -> &str {
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

pub async fn clean_zero_byte_files() -> Result<(), Error> {
    info!("Starting cover locks cleanup");
    let dir = CACHED_COVERS_DIR.clone();

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            let file = fs::File::open(&path)?;
            let size = file.metadata()?.len();
            if size == 0 {
                drop(file);
                info!("Found 0byte file at: '{}'", path.display());
                fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}

/// Returns the URI of a Show's cover directory given it's title.
fn get_cover_file_path(hash: &str) -> Result<std::path::PathBuf, Error> {
    let mut dir = CACHED_COVERS_DIR.clone();
    // Don't even bother setting a file extension as, we will
    // ultimately end up feeding it into the same loader regardless
    dir.push(hash);
    info!("Constructed cover path: '{}'", dir.display());
    Ok(dir)
}

fn determin_cover_path(pd: &ShowCoverModel) -> Result<PathBuf, Error> {
    // TODO: once we start storing them as strings
    // let hash = u64_to_vec_u8(pd.image_uri_hash().unwrap());
    let hash = format!("{}", pd.id());
    get_cover_file_path(&hash)
}

// FIXME: handle chunked downloads
async fn download_file(pd: ShowCoverModel, path: PathBuf) -> Result<(), DownloadError> {
    let tmp_dir = TempDir::new_in(&*PODCASTS_CACHE, &format!("cover-{}.part", pd.id()))?;

    let client = podcasts_data::downloader::client_builder().build()?;
    let response = client.get(pd.image_uri().unwrap()).send().await?;
    //FIXME: check for 200 or redirects, retry for 5xx
    debug!("Status Resp: {}", response.status());

    let filename = extract_file_extension(&response);
    let filename = tmp_dir.path().join(filename);
    info!("Downloading file into: '{:?}'", filename);
    let mut dest = tokio::fs::File::create(&filename).await?;

    let mut content = Cursor::new(response.bytes().await?);
    tokio::io::copy(&mut content, &mut dest).await?;

    dest.sync_all().await?;
    drop(dest);

    tokio::fs::rename(&filename, &path).await?;
    info!("Cached img into: '{}'", &path.display());

    Ok(())
}

async fn download_cover_image(pd: &ShowCoverModel, path: &PathBuf) -> Result<(), DownloadError> {
    let url = pd
        .image_uri()
        .ok_or(DownloadError::NoImageLocation)?
        .to_owned();

    if url.is_empty() {
        return Err(DownloadError::NoImageLocation);
    }

    // FIXME: Annoying ownership issues
    let pd = pd.clone();
    let path = path.clone();
    // FIXME: move the tokio spawn into the function
    // this needs tokio cause of reqwest
    crate::RUNTIME.spawn(async move { download_file(pd, path).await.unwrap() });
    Ok(())
}

pub async fn get_cover_file(
    pd: &ShowCoverModel,
    image: &WeakRef<gtk::Image>,
) -> Result<PathBuf, Error> {
    let cover = determin_cover_path(&pd)?;

    // Check if the cover is already downloaded and set it
    if !pd.is_cached_image_valid(&CACHE_VALID_DURATION) {
        info!("Removing expired cover cache: '{}'", cover.display());
        fs::remove_file(&cover)?;
    }

    if !cover.exists() {
        info!("Cover file does not exist, Starting download.");
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
        create_file_monitor(&cover, image).await?;
    } else {
        info!("Loading cover for '{}'", pd.title());
        let path = cover.to_str().unwrap().to_string();
        set_image_from_file_with_tokio(image, path).await.unwrap();
    }

    return Ok(cover);
}

// FIXME: this is ui code, doesn't really belong here
// FIXME: should be taking a WeakRef Image
// TODO: after loading, grab the Paintable and cache it to avoid creating
// multiple textures for the same files
pub fn load_image(image: &gtk::Image, podcast_id: i32) {
    crate::MAINCONTEXT.spawn_local_with_priority(
        glib::source::Priority::LOW,
        clone!(@weak image => async move {
            load_image_async(&image, podcast_id).await
        }),
    );
}

async fn load_image_async(image: &gtk::Image, podcast_id: i32) {
    use gtk::prelude::WidgetExt;
    use podcasts_data::dbqueries;

    let pd = gio::spawn_blocking(move || dbqueries::get_podcast_cover_from_id(podcast_id).unwrap())
        .await
        .unwrap();
    image.set_tooltip_text(Some(pd.title()));

    let path = crate::download_covers::get_cover_file(&pd, &image.downgrade())
        .await
        .unwrap();
    let path = path.to_str().unwrap().to_string();

    set_image_from_file_with_tokio(&image.downgrade(), path)
        .await
        .unwrap();
}

// FIMXE: Attach the Texture to the ShowCover Widget and load it from there
// FIMXE: Cache ShowCover Widgets and reuse them then
// FIXME: Weakrefs into async functions are weird
async fn set_image_from_file_with_tokio(
    image: &WeakRef<gtk::Image>,
    path: String,
) -> Result<(), Error> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    crate::RUNTIME.spawn(async move { tokio_make_texture(&path, sender).await });

    let image = match image.upgrade() {
        Some(i) => i,
        None => return Ok(()),
    };

    if let Ok(texture) = receiver.await {
        image.set_paintable(Some(&texture));
    }

    Ok(())
}

async fn tokio_make_texture(path: &str, sender: Sender<gdk::Texture>) {
    let r = COVER_TEXTURES.read().await;
    let texture = r.get(path).cloned();
    drop(r);

    if let Some(t) = texture {
        let _ = sender.send(t);
        return;
    };

    match gdk::Texture::from_filename(path) {
        Ok(texture) => {
            let mut w = COVER_TEXTURES.write().await;
            w.insert(path.to_string(), texture.clone());
            let _ = sender.send(texture);
        }
        Err(err) => {
            error!("Error: {}", err);
            error!("Failed to load texture from: {}", path);
        }
    }
}

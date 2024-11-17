use anyhow::{anyhow, Result};
use image::imageops::FilterType;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;

use crate::download_covers::determin_cover_path;
use podcasts_data::utils::get_cover_dir_path;
use podcasts_data::ShowCoverModel;
use std::sync::Arc;

// we only generate a fixed amount of thumbnails
// This enum is to avoid accidentally passing a thumb-size we didn't generate
#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum ThumbSize {
    Thumb64,
    Thumb128,
    Thumb256,
    Thumb512,
}
pub use self::ThumbSize::*;

impl ThumbSize {
    fn pixels(&self) -> u32 {
        match &self {
            Thumb64 => 64,
            Thumb128 => 128,
            Thumb256 => 256,
            Thumb512 => 512,
        }
    }
    pub fn hidpi(self, scale: i32) -> Option<ThumbSize> {
        // meh code
        if scale >= 5 {
            match self {
                Thumb64 => Some(Thumb512),
                Thumb128 => None,
                Thumb256 => None,
                Thumb512 => None,
            }
        } else if scale >= 3 {
            match self {
                Thumb64 => Some(Thumb256),
                Thumb128 => Some(Thumb512),
                Thumb256 => None,
                Thumb512 => None,
            }
        } else if scale >= 2 {
            match self {
                Thumb64 => Some(Thumb128),
                Thumb128 => Some(Thumb256),
                Thumb256 => Some(Thumb512),
                Thumb512 => None,
            }
        } else {
            Some(self)
        }
    }
}

impl Display for ThumbSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pixels())
    }
}

pub async fn generate(
    pd: &ShowCoverModel,
    path: &Path,
) -> Result<HashMap<ThumbSize, gtk::gdk::Texture>> {
    let sizes: [ThumbSize; 4] = [Thumb64, Thumb128, Thumb256, Thumb512];
    // All thumbs must generate, we rely on them existing if the main image exists.

    let path = path.to_path_buf();
    let image_full_size = crate::RUNTIME
        .spawn_blocking(move || {
            anyhow::Ok(Arc::new(
                image::ImageReader::open(path)?
                    .with_guessed_format()?
                    .decode()?,
            ))
        })
        .await??;

    let dir = get_cover_dir_path(pd.title());
    tokio::fs::create_dir_all(dir).await?;

    let handles: Vec<_> = sizes
        .into_iter()
        .map(|size| {
            let pixels = size.pixels();
            let thumb_path = determin_cover_path(pd, Some(size));
            let image_full_size = image_full_size.clone();

            crate::RUNTIME.spawn(async move {
                let tmp_path = thumb_path.with_extension(".part");
                let tmp_path2 = tmp_path.clone();
                // save and read gdk texture
                let texture = crate::RUNTIME
                    .spawn_blocking(move || {
                        let image = image_full_size.resize(pixels, pixels, FilterType::Lanczos3);
                        image.save_with_format(&tmp_path2, image::ImageFormat::Png)?;
                        gtk::gdk::Texture::from_filename(&tmp_path2)
                            .map_err(|_| anyhow!("failed to read gtk texture"))
                    })
                    .await??;
                tokio::fs::rename(&tmp_path, &thumb_path).await?;
                Ok((size, texture))
            })
        })
        .collect();
    let result: Result<HashMap<ThumbSize, gtk::gdk::Texture>> =
        futures_util::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap_or(Err(anyhow!("Failed to write cover thumbnail."))))
            .collect();
    result
}

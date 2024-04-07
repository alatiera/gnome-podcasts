use anyhow::{anyhow, Context, Result};
use gtk::glib;
use gtk::prelude::*;
use std::collections::HashMap;
use std::fmt::Display;

use crate::download_covers::determin_cover_path;
use podcasts_data::ShowCoverModel;

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
    fn pixels(&self) -> f32 {
        match &self {
            Thumb64 => 64.0,
            Thumb128 => 128.0,
            Thumb256 => 256.0,
            Thumb512 => 512.0,
        }
    }
    pub fn hidpi(self, scale: i32) -> Option<ThumbSize> {
        if scale >= 2 {
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
        write!(f, "{}", self.pixels() as i32)
    }
}

fn scaled_texture(
    renderer: &gtk::gsk::Renderer,
    full_texture: &gtk::gdk::Texture,
    size: &ThumbSize,
) -> Result<gtk::gdk::Texture> {
    let snapshot = gtk::Snapshot::new();
    let pixels = size.pixels();
    snapshot.append_scaled_texture(
        full_texture,
        gtk::gsk::ScalingFilter::Trilinear,
        &gtk::graphene::Rect::new(0.0, 0.0, pixels, pixels),
    );

    let node = snapshot
        .to_node()
        .context("can't turn snapshot into node")?;

    Ok(renderer.render_texture(node, None))
}

fn render_thumbs(texture: &gtk::gdk::Texture) -> Result<HashMap<ThumbSize, gtk::gdk::Texture>> {
    let display = gtk::gdk::Display::default().context("can't get a display")?;
    let surface = gtk::gdk::Surface::new_toplevel(&display);
    let renderer = gtk::gsk::Renderer::for_surface(&surface).context("no renderer")?;
    if !renderer.is_realized() {
        renderer
            .realize_for_display(&display)
            .context("Failed to realize renderer")?;
    }

    let sizes: [ThumbSize; 4] = [Thumb64, Thumb128, Thumb256, Thumb512];
    // All thumbs must generate, we rely on them existing if the main image exists.
    let thumbs: Result<HashMap<_, _>> = sizes
        .into_iter()
        .map(|size| {
            let thumb_texture = scaled_texture(&renderer, texture, &size)?;
            Ok((size, thumb_texture))
        })
        .collect();

    if renderer.is_realized() {
        renderer.unrealize();
    }
    thumbs
}

async fn write_thumbs(
    pd: &ShowCoverModel,
    thumbs: &HashMap<ThumbSize, gtk::gdk::Texture>,
) -> Result<()> {
    let handles: Vec<_> = thumbs
        .iter()
        .map(|(size, texture)| {
            let thumb_path = determin_cover_path(&pd, Some(size.clone()));
            let bytes = texture.save_to_png_bytes(); // must be read on gtk thread
            crate::RUNTIME.spawn(async move {
                let tmp_path = thumb_path.with_extension(".part");
                let mut dest = tokio::fs::File::create(&tmp_path).await?;
                let mut content = std::io::Cursor::new(bytes);
                tokio::io::copy(&mut content, &mut dest).await?;
                dest.sync_all().await?;
                tokio::fs::rename(&tmp_path, &thumb_path).await?;
                Ok::<(), anyhow::Error>(())
            })
        })
        .collect();
    // TODO can this error be unwrapped cleaner????
    let result: Result<Vec<_>> = futures_util::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(Err(anyhow!("Failed to write cover thumbnail."))))
        .collect();
    result?;
    Ok(())
}

/// Must run on gtk thread
pub async fn generate(
    pd: &ShowCoverModel,
    texture: gtk::gdk::Texture,
) -> Result<HashMap<ThumbSize, gtk::gdk::Texture>> {
    let thumbs = render_thumbs(&texture)?;
    write_thumbs(&pd, &thumbs).await?;
    Ok(thumbs)
}

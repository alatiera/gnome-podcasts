use anyhow::{anyhow, Context, Result};
use std::fmt::Display;

use gtk::glib;
use gtk::prelude::*;

use crate::download_covers::determin_cover_path;
use podcasts_data::ShowCoverModel;

// we only generate a fixed amount of thumbnails
// This enum is to avoid accidentally passing a thumb-size we didn't generate
#[derive(Clone)]
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
    texture: gtk::gdk::Texture,
    size: &ThumbSize,
) -> Result<gtk::gdk::Texture> {
    let snapshot = gtk::Snapshot::new();
    let pixels = size.pixels();
    snapshot.append_scaled_texture(
        &texture,
        gtk::gsk::ScalingFilter::Trilinear,
        &gtk::graphene::Rect::new(0.0, 0.0, pixels, pixels),
    );

    let node = snapshot
        .to_node()
        .context("can't turn snapshot into node")?;

    Ok(renderer.render_texture(node, None))
}

pub async fn generate(pd: &ShowCoverModel, texture: gtk::gdk::Texture) -> Result<()> {
    let pd = pd.clone();
    crate::MAINCONTEXT
        .spawn_with_priority(glib::source::Priority::DEFAULT_IDLE, async move {
            let sizes: [ThumbSize; 4] = [Thumb64, Thumb128, Thumb256, Thumb512];

            let thumbs = {
                let display = gtk::gdk::Display::default().context("can't get a display")?;
                let surface = gtk::gdk::Surface::new_toplevel(&display);
                let renderer = gtk::gsk::Renderer::for_surface(&surface).context("no renderer")?;
                if !renderer.is_realized() {
                    let _ = renderer
                        .realize_for_display(&display)
                        .context("Failed to realize renderer")?;
                }

                let thumbs: Vec<(ThumbSize, gtk::gdk::Texture)> = sizes
                    .iter()
                    .map(|size| {
                        (
                            size.clone(),
                            scaled_texture(&renderer, texture.clone(), size).unwrap(),
                        )
                    })
                    .collect();

                if renderer.is_realized() {
                    renderer.unrealize();
                }
                thumbs
            };

            let handles: Vec<_> = thumbs
                .into_iter()
                .map(|(size, texture)| {
                    let thumb_path = determin_cover_path(&pd, Some(size));
                    let bytes = texture.save_to_png_bytes();
                    crate::RUNTIME.spawn(async move {
                        let mut dest = tokio::fs::File::create(&thumb_path).await?;
                        let mut content = std::io::Cursor::new(bytes);
                        tokio::io::copy(&mut content, &mut dest).await?;
                        dest.sync_all().await?;
                        drop(dest);
                        Ok::<(), anyhow::Error>(())
                    })
                })
                .collect();
            futures_util::future::join_all(handles).await;
            Ok(())
        })
        .await
        .unwrap_or(Err(anyhow!("Failed to render snapshot on main thread")))
}

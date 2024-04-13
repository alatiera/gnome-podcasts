// cover_image.rs
//
// Copyright 2024 nee <nee-git@patchouli.garden>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use crate::download_covers::load_image;
use podcasts_data::errors::DownloadError;

#[derive(Default)]
pub struct CoverImagePriv {
    image: gtk::Image,
}

#[glib::object_subclass]
impl ObjectSubclass for CoverImagePriv {
    const NAME: &'static str = "PdCoverImage";
    type Type = super::CoverImage;
    type ParentType = adw::Bin;
}

impl ObjectImpl for CoverImagePriv {
    fn constructed(&self) {
        self.parent_constructed();

        self.obj()
            .bind_property("width_request", &self.image, "pixel_size")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        // Must pass on classes for rounded borders on images to work.
        self.obj()
            .bind_property("css_classes", &self.image, "css_classes")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        self.image.set_icon_name(Some("image-x-generic-symbolic"));
        self.image.set_overflow(gtk::Overflow::Hidden);
        self.obj().set_child(Some(&self.image));
    }
}

impl WidgetImpl for CoverImagePriv {}
impl BinImpl for CoverImagePriv {}

glib::wrapper! {
    pub struct CoverImage(ObjectSubclass<CoverImagePriv>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl CoverImage {
    pub(crate) fn new(width: i32) -> Self {
        let widget: Self = glib::Object::new();
        widget.set_width_request(width);
        widget
    }

    pub(crate) fn init(&self, show_id: i32, size: crate::thumbnail_generator::ThumbSize) {
        // TODO Surface has scale() fn that returns a f64 dpi-scale, maybe use that?
        // TODO maybe load the full size image when bigger than 512 is requested?
        let size = size
            .hidpi(self.imp().image.scale_factor())
            .unwrap_or(crate::Thumb512);
        let image = self.imp().image.downgrade();
        crate::MAINCONTEXT.spawn_local_with_priority(glib::source::Priority::LOW, async move {
            if let Err(err) = load_image(&image, show_id, size).await {
                if let Some(DownloadError::NoLongerNeeded) = err.downcast_ref::<DownloadError>() {
                    // weak image reference couldn't be upgraded, no need to print this
                    return;
                }
                error!("Failed to load image: {err}");
            }
        });
    }
}

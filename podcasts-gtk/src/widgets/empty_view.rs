// empty_view.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
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

use crate::config::APP_ID;

use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

#[derive(Debug, CompositeTemplate)]
#[template(resource = "/org/gnome/Podcasts/gtk/empty_view.ui")]
pub struct EmptyViewPriv {
    #[template_child]
    pub status_page: TemplateChild<adw::StatusPage>,
}

#[glib::object_subclass]
impl ObjectSubclass for EmptyViewPriv {
    const NAME: &'static str = "PdEmptyView";
    type Type = EmptyView;
    type ParentType = adw::Bin;

    fn new() -> Self {
        Self {
            status_page: TemplateChild::default(),
        }
    }

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    // You must call `Widget`'s `init_template()` within `instance_init()`.
    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for EmptyViewPriv {
    fn constructed(&self) {
        self.parent_constructed();
        self.status_page.set_icon_name(Some(APP_ID));
    }
}

impl WidgetImpl for EmptyViewPriv {}
impl BinImpl for EmptyViewPriv {}

glib::wrapper! {
    pub struct EmptyView(ObjectSubclass<EmptyViewPriv>)
        @extends gtk::Widget, adw::Bin;
}

impl Default for EmptyView {
    fn default() -> Self {
        glib::Object::new()
    }
}

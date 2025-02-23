// empty_show.rs
//
// Copyright 2022 Jordan Petridis <jpetridis@gnome.org>
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

use gtk::subclass::prelude::*;
use gtk::{CompositeTemplate, glib};

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/gnome/Podcasts/gtk/empty_show.ui")]
pub struct EmptyShowPriv {}

#[glib::object_subclass]
impl ObjectSubclass for EmptyShowPriv {
    const NAME: &'static str = "PdEmptyShow";
    type Type = EmptyShow;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    // You must call `Widget`'s `init_template()` within `instance_init()`.
    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for EmptyShowPriv {}

impl WidgetImpl for EmptyShowPriv {}
impl BoxImpl for EmptyShowPriv {}

glib::wrapper! {
    pub struct EmptyShow(ObjectSubclass<EmptyShowPriv>)
        @extends gtk::Widget, gtk::Box;
}

impl Default for EmptyShow {
    fn default() -> Self {
        glib::Object::new()
    }
}

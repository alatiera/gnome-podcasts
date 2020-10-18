// empty.rs
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

use std::ops::Deref;

#[derive(Clone, Debug)]
pub(crate) struct EmptyView(gtk::Box);

impl Deref for EmptyView {
    type Target = gtk::Box;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for EmptyView {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/empty_view.ui");
        let view: gtk::Box = builder.object("empty_view").unwrap();
        let image: gtk::Image = builder.object("image").unwrap();
        image.set_from_icon_name(Some(format!("{}-symbolic", APP_ID).as_str()));
        EmptyView(view)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EmptyShow(gtk::Box);

impl Deref for EmptyShow {
    type Target = gtk::Box;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for EmptyShow {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/empty_view.ui");
        let box_: gtk::Box = builder.object("empty_show").unwrap();
        EmptyShow(box_)
    }
}

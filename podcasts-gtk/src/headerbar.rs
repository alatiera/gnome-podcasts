// headerbar.rs
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

use crate::app::Action;
use async_channel::Sender;
use gtk::gio;
use gtk::glib::clone;
use gtk::prelude::*;
use std::rc::Rc;

use crate::stacks::Content;

#[derive(Debug, Clone)]
// TODO: Make a proper state machine for the headerbar states
pub(crate) struct Header {
    pub(crate) container: adw::HeaderBar,
    pub(crate) switch: adw::ViewSwitcher,
    add: gtk::Button,
}

impl Default for Header {
    fn default() -> Header {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/headerbar.ui");
        let menus = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/hamburger.ui");

        let header = builder.object("headerbar").unwrap();
        let switch: adw::ViewSwitcher = builder.object("switch").unwrap();

        // The hamburger menu
        let hamburger: gtk::MenuButton = builder.object("hamburger").unwrap();
        let app_menu: gio::MenuModel = menus.object("menu").unwrap();
        hamburger.set_menu_model(Some(&app_menu));

        let add = builder.object("add_button").unwrap();
        Header {
            container: header,
            switch,
            add,
        }
    }
}

impl Header {
    pub(crate) fn new(content: &Content, sender: &Sender<Action>) -> Rc<Self> {
        let h = Rc::new(Header::default());
        Self::init(&h, content, sender);
        h
    }

    pub(crate) fn init(s: &Rc<Self>, content: &Content, sender: &Sender<Action>) {
        s.switch.set_stack(Some(&content.get_stack()));
        s.add.connect_clicked(clone!(@strong sender => move |_| {
            send_blocking!(sender, Action::GoToDiscovery);
        }));
    }
}

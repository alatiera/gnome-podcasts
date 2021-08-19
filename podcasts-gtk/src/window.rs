// window.rs
//
// Copyright 2019 Jordan Petridis <jpetridis@gnome.org>
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

use glib::clone;
use glib::Sender;

use gio::{self, prelude::*, SettingsExt};

use gtk::prelude::*;

use libhandy as hdy;
use libhandy::DeckExt;

use crate::app::{Action, PdApplication};
use crate::headerbar::Header;
use crate::settings::{self, WindowGeometry};
use crate::stacks::Content;
use crate::utils::{self, make_action};
use crate::widgets::about_dialog;
use crate::widgets::appnotif::InAppNotification;
use crate::widgets::player;

use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use crate::config::APP_ID;
use crate::i18n::i18n;

#[derive(Debug)]
pub struct MainWindow {
    pub(crate) window: hdy::ApplicationWindow,
    pub(crate) overlay: gtk::Overlay,
    pub(crate) content: Rc<Content>,
    pub(crate) headerbar: Rc<Header>,
    pub(crate) player: player::PlayerWrapper,
    pub(crate) main_deck: libhandy::Deck,
    pub(crate) updating: Cell<bool>,
    pub(crate) updater: RefCell<Option<InAppNotification>>,
    pub(crate) sender: Sender<Action>,
}

impl MainWindow {
    pub(crate) fn new(app: &PdApplication, sender: &Sender<Action>) -> Self {
        let settings = gio::Settings::new(APP_ID);

        let window = hdy::ApplicationWindow::new();
        window.set_application(Some(app));

        window.set_title(&i18n("Podcasts"));
        if APP_ID.ends_with("Devel") {
            window.get_style_context().add_class("devel");
        }

        window.connect_delete_event(
            clone!(@strong settings, @weak app => @default-return Inhibit(false), move |window, _| {
                    info!("Saving window position");
                    WindowGeometry::from_window(&window).write(&settings);

                    info!("Application is exiting");
                    let app = app.upcast::<gio::Application>();
                    app.quit();
                    Inhibit(false)
            }),
        );

        // Create a content instance
        let content = Content::new(&sender).expect("Content initialization failed.");

        // Create the headerbar
        let header = Header::new(&content, &sender);

        // Add the content main stack to the overlay.
        let overlay = gtk::Overlay::new();
        let main_deck = libhandy::Deck::new();
        main_deck.set_can_swipe_forward(false);
        main_deck.add(&content.get_container());
        overlay.add(&main_deck);

        let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);

        // Add the Headerbar to the window.
        content
            .get_container()
            .pack_start(&header.container, false, true, 0);

        // Add the overlay to the main Box
        wrap.add(&overlay);

        let player = player::PlayerWrapper::new(&sender);
        // Add the player to the main Box
        wrap.add(&player.borrow().container);

        wrap.add(&header.bottom_switcher);

        window.add(&wrap);

        // Retrieve the previous window position and size.
        WindowGeometry::from_settings(&settings).apply(&window);

        // Update the feeds right after the Window is initialized.
        if settings.get_boolean("refresh-on-startup") {
            info!("Refresh on startup.");
            utils::schedule_refresh(None, sender.clone());
        }

        let refresh_interval = settings::get_refresh_interval(&settings).num_seconds() as u32;
        info!("Auto-refresh every {:?} seconds.", refresh_interval);

        glib::timeout_add_seconds_local(
            refresh_interval,
            clone!(@strong sender => move || {
                    utils::schedule_refresh(None, sender.clone());
                    glib::Continue(true)
            }),
        );

        Self {
            window,
            overlay,
            headerbar: header,
            content,
            player,
            main_deck,
            updating: Cell::new(false),
            updater: RefCell::new(None),
            sender: sender.clone(),
        }
    }

    /// Define the `GAction`s.
    ///
    /// Used in menus and the keyboard shortcuts dialog.
    pub fn setup_gactions(&self) {
        let sender = &self.sender;
        // Create the `refresh` action.
        //
        // This will trigger a refresh of all the shows in the database.
        make_action(
            &self.window,
            "refresh",
            clone!(@strong sender => move |_, _| {
                    glib::idle_add_local(
                        clone!(@strong sender => move || {
                            utils::schedule_refresh(None, sender.clone());
                            glib::Continue(false)
                }));
            }),
        );

        // Create the `OPML` import action
        make_action(
            &self.window,
            "import",
            clone!(@strong sender, @weak self.window as window => move |_, _| {
                    utils::on_import_clicked(&window.upcast(), &sender);
            }),
        );

        make_action(
            &self.window,
            "export",
            clone!(@strong sender, @weak self.window as window => move |_, _| {
                    utils::on_export_clicked(&window.upcast(), &sender);
            }),
        );

        // Create the action that shows a `gtk::AboutDialog`
        make_action(
            &self.window,
            "about",
            clone!(@weak self.window as win => move |_, _| {
                    about_dialog(&win.upcast());
            }),
        );

        // Create the menu actions
        make_action(
            &self.window,
            "menu",
            clone!(@weak self.headerbar as headerbar => move |_, _| {
                    headerbar.open_menu();
            }),
        );
    }
    /// Remove all items from the `main_deck` except from the content
    pub fn clear_deck(&self) {
        for c in self.main_deck.get_children() {
            if c.get_widget_name() != "content" {
                self.main_deck.remove(&c);
            }
        }
    }
}

impl Deref for MainWindow {
    type Target = hdy::ApplicationWindow;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

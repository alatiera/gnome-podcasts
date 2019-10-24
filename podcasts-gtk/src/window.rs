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

use glib;
use glib::Variant;

use gio::{self, prelude::*, ActionMapExt, SettingsExt};

use gtk;
use gtk::prelude::*;

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::app::{Action, PdApplication};
use crate::headerbar::Header;
use crate::settings::{self, WindowGeometry};
use crate::stacks::Content;
use crate::utils;
use crate::widgets::about_dialog;
use crate::widgets::appnotif::InAppNotification;
use crate::widgets::player;

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use crate::config::APP_ID;
use crate::i18n::i18n;

/// Creates an action named `name` in the action map `T with the handler `F`
fn action<T, F>(thing: &T, name: &str, action: F)
where
    T: ActionMapExt,
    for<'r, 's> F: Fn(&'r gio::SimpleAction, Option<&Variant>) + 'static,
{
    // Create a stateless, parameterless action
    let act = gio::SimpleAction::new(name, None);
    // Connect the handler
    act.connect_activate(action);
    // Add it to the map
    thing.add_action(&act);
}

#[derive(Debug)]
pub struct MainWindow {
    app: PdApplication,
    pub(crate) window: gtk::ApplicationWindow,
    pub(crate) overlay: gtk::Overlay,
    pub(crate) content: Rc<Content>,
    pub(crate) headerbar: Rc<Header>,
    pub(crate) player: player::PlayerWrapper,
    pub(crate) updater: RefCell<Option<InAppNotification>>,
    pub(crate) sender: Sender<Action>,
    pub(crate) receiver: Receiver<Action>,
}

impl MainWindow {
    pub fn new(app: &PdApplication) -> Self {
        let settings = gio::Settings::new("org.gnome.Podcasts");

        let (sender, receiver) = unbounded();

        let window = gtk::ApplicationWindow::new(app);
        window.set_title(&i18n("Podcasts"));
        if APP_ID.ends_with("Devel") {
            window.get_style_context().add_class("devel");
        }

        let weak_s = settings.downgrade();
        let weak_app = app.downgrade();
        window.connect_delete_event(move |window, _| {
            let app = match weak_app.upgrade() {
                Some(a) => a,
                None => return Inhibit(false),
            };

            let settings = match weak_s.upgrade() {
                Some(s) => s,
                None => return Inhibit(false),
            };

            info!("Saving window position");
            WindowGeometry::from_window(&window).write(&settings);

            info!("Application is exiting");
            let app = app.clone().upcast::<gio::Application>();
            app.quit();
            Inhibit(false)
        });

        // Create a content instance
        let content = Content::new(&sender).expect("Content initialization failed.");

        // Create the headerbar
        let header = Header::new(&content, &sender);
        // Add the Headerbar to the window.
        window.set_titlebar(Some(&header.container));

        // Add the content main stack to the overlay.
        let overlay = gtk::Overlay::new();
        overlay.add(&content.get_stack());

        let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);
        // Add the overlay to the main Box
        wrap.add(&overlay);

        let player = player::PlayerWrapper::new(&sender);
        // Add the player to the main Box
        wrap.add(&player.action_bar);

        wrap.add(&header.bottom_switcher);

        let updater = RefCell::new(None);

        window.add(&wrap);

        // Retrieve the previous window position and size.
        WindowGeometry::from_settings(&settings).apply(&window);

        // Update the feeds right after the Window is initialized.
        if settings.get_boolean("refresh-on-startup") {
            info!("Refresh on startup.");
            let s: Option<Vec<_>> = None;
            utils::refresh(s, sender.clone());
        }

        let refresh_interval = settings::get_refresh_interval(&settings).num_seconds() as u32;
        info!("Auto-refresh every {:?} seconds.", refresh_interval);

        let r_sender = sender.clone();
        gtk::timeout_add_seconds(refresh_interval, move || {
            let s: Option<Vec<_>> = None;
            utils::refresh(s, r_sender.clone());

            glib::Continue(true)
        });

        Self {
            app: app.clone(),
            window,
            overlay,
            headerbar: header,
            content,
            player,
            updater,
            sender,
            receiver,
        }
    }

    /// Define the `GAction`s.
    ///
    /// Used in menus and the keyboard shortcuts dialog.
    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn setup_gactions(&self) {
        let sender = &self.sender;
        let weak_win = self.window.downgrade();

        // Create the `refresh` action.
        //
        // This will trigger a refresh of all the shows in the database.
        action(&self.window, "refresh", clone!(sender => move |_, _| {
            gtk::idle_add(clone!(sender => move || {
                let s: Option<Vec<_>> = None;
                utils::refresh(s, sender.clone());
                glib::Continue(false)
            }));
        }));
        self.app.set_accels_for_action("win.refresh", &["<primary>r"]);

        // Create the `OPML` import action
        action(&self.window, "import", clone!(sender, weak_win => move |_, _| {
            weak_win.upgrade().map(|win| utils::on_import_clicked(&win, &sender));
        }));

        action(&self.window, "export", clone!(sender, weak_win => move |_, _| {
            weak_win.upgrade().map(|win| utils::on_export_clicked(&win, &sender));
        }));

        // Create the action that shows a `gtk::AboutDialog`
        action(&self.window, "about", clone!(weak_win => move |_, _| {
            weak_win.upgrade().map(|win| about_dialog(&win));
        }));

        // Create the quit action
        let weak_instance = self.app.downgrade();
        action(&self.window, "quit", move |_, _| {
            weak_instance.upgrade().map(|app| app.quit());
        });
        self.app.set_accels_for_action("win.quit", &["<primary>q"]);

        // Create the menu action
        let header = Rc::downgrade(&self.headerbar);
        action(&self.window, "menu", move |_, _| {
            header.upgrade().map(|h| h.open_menu());
        });
        // Bind the hamburger menu button to `F10`
        self.app.set_accels_for_action("win.menu", &["F10"]);
    }
}

impl Deref for MainWindow {
    type Target = gtk::ApplicationWindow;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

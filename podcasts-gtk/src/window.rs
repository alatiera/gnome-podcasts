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
use glib::clone;
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

use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use crate::config::APP_ID;
use crate::i18n::i18n;

/// Creates an action named `name` in the action map `T with the handler `F`
fn action<T, F>(thing: &T, name: &str, action: F)
where
    T: ActionMapExt,
    F: Fn(&gio::SimpleAction, Option<&Variant>) + 'static,
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
    pub(crate) updating: Cell<bool>,
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
        wrap.add(&player.container);

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

        gtk::timeout_add_seconds(
            refresh_interval,
            clone!(@strong sender => move || {
                    utils::schedule_refresh(None, sender.clone());
                    glib::Continue(true)
            }),
        );

        Self {
            app: app.clone(),
            window,
            overlay,
            headerbar: header,
            content,
            player,
            updating: Cell::new(false),
            updater: RefCell::new(None),
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
        // Create the `refresh` action.
        //
        // This will trigger a refresh of all the shows in the database.
        action(&self.window, "refresh", 
            clone!(@strong sender => move |_, _| {
                gtk::idle_add(
                    clone!(@strong sender => move || {
                        utils::schedule_refresh(None, sender.clone());
                        glib::Continue(false)
            }));
        }));
        self.app.set_accels_for_action("win.refresh", &["<primary>r"]);

        // Create the `OPML` import action
        action(&self.window, "import", 
            clone!(@strong sender, @weak self.window as window => move |_, _| {
                utils::on_import_clicked(&window, &sender);
        }));

        action(&self.window, "export", 
            clone!(@strong sender, @weak self.window as window => move |_, _| {
                utils::on_export_clicked(&window, &sender);
        }));

        // Create the action that shows a `gtk::AboutDialog`
        action(&self.window, "about", 
            clone!(@weak self.window as win => move |_, _| {
                about_dialog(&win);
        }));

        // Create the quit action
        action(&self.window, "quit", 
            clone!(@weak self.app as app => move |_, _| {
                app.quit();
        }));
        self.app.set_accels_for_action("win.quit", &["<primary>q"]);

        // Create the menu actions
        action(&self.window, "menu",
            clone!(@weak self.headerbar as headerbar => move |_, _| {
                headerbar.open_menu();
        }));
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

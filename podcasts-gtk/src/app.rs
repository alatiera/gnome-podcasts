// app.rs
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

#![allow(clippy::new_without_default)]

use glib::subclass::prelude::*;
use glib::subclass::simple::{ClassStruct, InstanceStruct};
use glib::translate::*;
use glib::{glib_object_impl, glib_object_subclass, glib_object_wrapper, glib_wrapper};

use gio::subclass::application::ApplicationImplExt;
use gio::{self, prelude::*, ActionMapExt, ApplicationFlags, SettingsExt};

use gtk;
use gtk::prelude::*;

use gettextrs::{bindtextdomain, setlocale, textdomain, LocaleCategory};

use crossbeam_channel::Receiver;
use fragile::Fragile;
use podcasts_data::Show;

use crate::settings;
use crate::stacks::PopulatedState;
use crate::utils;
use crate::widgets::appnotif::{InAppNotification, SpinnerState, State};
use crate::widgets::show_menu::{mark_all_notif, remove_show_notif, ShowMenu};
use crate::window::MainWindow;

use std::cell::RefCell;
use std::env;
use std::sync::Arc;

use crate::config::{APP_ID, LOCALEDIR};
use crate::i18n::i18n;

pub struct PdApplicationPrivate {
    window: RefCell<Option<MainWindow>>,
    settings: RefCell<Option<gio::Settings>>,
}

impl ObjectSubclass for PdApplicationPrivate {
    const NAME: &'static str = "PdApplication";
    type ParentType = gtk::Application;
    type Instance = InstanceStruct<Self>;
    type Class = ClassStruct<Self>;

    glib_object_subclass!();

    fn new() -> Self {
        Self {
            window: RefCell::new(None),
            settings: RefCell::new(None),
        }
    }
}

impl ObjectImpl for PdApplicationPrivate {
    glib_object_impl!();
}

impl gio::subclass::prelude::ApplicationImpl for PdApplicationPrivate {
    fn activate(&self, app: &gio::Application) {
        debug!("GtkApplication<PdApplication>::activate");

        if let Some(ref window) = *self.window.borrow() {
            // Ideally Gtk4/GtkBuilder make this irrelvent
            window.show_all();
            window.present();
            info!("Window presented");
            return;
        }

        let app = app.clone().downcast::<PdApplication>().expect("How?");
        let window = MainWindow::new(&app);
        window.setup_gactions();
        window.show_all();
        window.present();
        self.window.replace(Some(window));
        // Setup the Action channel
        gtk::timeout_add(25, clone!(app => move || app.setup_action_channel()));
    }

    fn startup(&self, app: &gio::Application) {
        debug!("GtkApplication<PdApplication>::startup");

        self.parent_startup(app);

        let settings = gio::Settings::new("org.gnome.Podcasts");

        let cleanup_date = settings::get_cleanup_date(&settings);
        // Garbage collect watched episodes from the disk
        utils::cleanup(cleanup_date);

        self.settings.replace(Some(settings));

        let app = app.clone().downcast::<PdApplication>().expect("How?");
        app.setup_timed_callbacks();
    }
}

impl gtk::subclass::application::GtkApplicationImpl for PdApplicationPrivate {}

glib_wrapper! {
    pub struct PdApplication(Object<InstanceStruct<PdApplicationPrivate>, ClassStruct<PdApplicationPrivate>, PdApplicationClass>) @extends gio::Application, gtk::Application;

    match fn {
        get_type => || PdApplicationPrivate::get_type().to_glib(),
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Action {
    RefreshAllViews,
    RefreshEpisodesView,
    RefreshEpisodesViewBGR,
    RefreshShowsView,
    ReplaceWidget(Arc<Show>),
    RefreshWidgetIfSame(i32),
    ShowWidgetAnimated,
    ShowShowsAnimated,
    HeaderBarShowTile(String),
    HeaderBarNormal,
    MarkAllPlayerNotification(Arc<Show>),
    ShowUpdateNotif(Receiver<bool>),
    RemoveShow(Arc<Show>),
    ErrorNotification(String),
    InitEpisode(i32),
    InitShowMenu(Fragile<ShowMenu>),
    EmptyState,
    PopulatedState,
    RaiseWindow,
}

impl PdApplication {
    pub(crate) fn new() -> Self {
        let application = glib::Object::new(
            PdApplication::static_type(),
            &[
                ("application-id", &Some(APP_ID)),
                ("flags", &ApplicationFlags::empty()),
            ],
        )
        .expect("Application initialization failed...")
        .downcast::<PdApplication>()
        .expect("Congrats, you have won a prize for triggering an impossible outcome");

        application.set_resource_base_path(Some("/org/gnome/Podcasts"));

        application
    }

    fn setup_timed_callbacks(&self) {
        self.setup_dark_theme();
    }

    fn setup_dark_theme(&self) {
        let data = PdApplicationPrivate::from_instance(self);
        if let Some(ref settings) = *data.settings.borrow() {
            let gtk_settings = gtk::Settings::get_default().unwrap();
            settings.bind(
                "dark-theme",
                &gtk_settings,
                "gtk-application-prefer-dark-theme",
                gio::SettingsBindFlags::DEFAULT,
            );
        } else {
            debug_assert!(false, "Well how'd you manage that?");
        }
    }

    fn setup_action_channel(&self) -> glib::Continue {
        use crossbeam_channel::TryRecvError;
        let data = PdApplicationPrivate::from_instance(self);

        if let Some(ref window) = *data.window.borrow() {
            let action = match window.receiver.try_recv() {
                Ok(a) => a,
                Err(TryRecvError::Empty) => return glib::Continue(true),
                Err(TryRecvError::Disconnected) => {
                    unreachable!("How the hell was the action channel dropped.")
                }
            };

            trace!("Incoming channel action: {:?}", action);
            match action {
                Action::RefreshAllViews => window.content.update(),
                Action::RefreshShowsView => window.content.update_shows_view(),
                Action::RefreshWidgetIfSame(id) => window.content.update_widget_if_same(id),
                Action::RefreshEpisodesView => window.content.update_home(),
                Action::RefreshEpisodesViewBGR => window.content.update_home_if_background(),
                Action::ReplaceWidget(pd) => {
                    let shows = window.content.get_shows();
                    let pop = shows.borrow().populated();
                    pop.borrow_mut()
                        .replace_widget(pd.clone())
                        .map_err(|err| error!("Failed to update ShowWidget: {}", err))
                        .map_err(|_| error!("Failed to update ShowWidget {}", pd.title()))
                        .ok();
                }
                Action::ShowWidgetAnimated => {
                    let shows = window.content.get_shows();
                    let pop = shows.borrow().populated();
                    pop.borrow_mut().switch_visible(
                        PopulatedState::Widget,
                        gtk::StackTransitionType::SlideLeft,
                    );
                }
                Action::ShowShowsAnimated => {
                    let shows = window.content.get_shows();
                    let pop = shows.borrow().populated();
                    pop.borrow_mut()
                        .switch_visible(PopulatedState::View, gtk::StackTransitionType::SlideRight);
                }
                Action::HeaderBarShowTile(title) => window.headerbar.switch_to_back(&title),
                Action::HeaderBarNormal => window.headerbar.switch_to_normal(),
                Action::MarkAllPlayerNotification(pd) => {
                    let notif = mark_all_notif(pd, &window.sender);
                    notif.show(&window.overlay);
                }
                Action::RemoveShow(pd) => {
                    let notif = remove_show_notif(pd, window.sender.clone());
                    notif.show(&window.overlay);
                }
                Action::ErrorNotification(err) => {
                    error!("An error notification was triggered: {}", err);
                    let callback = |revealer: gtk::Revealer| {
                        revealer.set_reveal_child(false);
                        glib::Continue(false)
                    };
                    let undo_cb: Option<fn()> = None;
                    let notif = InAppNotification::new(&err, 6000, callback, undo_cb);
                    notif.show(&window.overlay);
                }
                Action::ShowUpdateNotif(receiver) => {
                    let sender = window.sender.clone();
                    let callback = move |revealer: gtk::Revealer| match receiver.try_recv() {
                        Err(TryRecvError::Empty) => glib::Continue(true),
                        Err(TryRecvError::Disconnected) => glib::Continue(false),
                        Ok(_) => {
                            revealer.set_reveal_child(false);
                            sender
                                .send(Action::RefreshAllViews)
                                .expect("Action channel blew up somehow");
                            glib::Continue(false)
                        }
                    };
                    let txt = i18n("Fetching new episodes");
                    let undo_cb: Option<fn()> = None;
                    let updater = InAppNotification::new(&txt, 250, callback, undo_cb);
                    updater.set_close_state(State::Hidden);
                    updater.set_spinner_state(SpinnerState::Active);

                    let old = window.updater.replace(Some(updater));
                    old.map(|i| i.destroy());
                    window
                        .updater
                        .borrow()
                        .as_ref()
                        .map(|i| i.show(&window.overlay));
                }
                Action::InitEpisode(rowid) => {
                    let res = window.player.initialize_episode(rowid);
                    debug_assert!(res.is_ok());
                }
                Action::InitShowMenu(s) => {
                    let menu = &s.get().container;
                    window.headerbar.set_secondary_menu(menu);
                }
                Action::EmptyState => {
                    window
                        .window
                        .lookup_action("refresh")
                        .and_then(|action| action.downcast::<gio::SimpleAction>().ok())
                        // Disable refresh action
                        .map(|action| action.set_enabled(false));

                    window.headerbar.switch.set_sensitive(false);
                    window.content.switch_to_empty_views();
                }
                Action::PopulatedState => {
                    window
                        .window
                        .lookup_action("refresh")
                        .and_then(|action| action.downcast::<gio::SimpleAction>().ok())
                        // Enable refresh action
                        .map(|action| action.set_enabled(true));

                    window.headerbar.switch.set_sensitive(true);
                    window.content.switch_to_populated();
                }
                Action::RaiseWindow => window.window.present(),
            };
        } else {
            debug_assert!(false, "Huh that's odd then");
        }

        glib::Continue(true)
    }

    pub(crate) fn run() {
        // Set up the textdomain for gettext
        setlocale(LocaleCategory::LcAll, "");
        bindtextdomain("gnome-podcasts", LOCALEDIR);
        textdomain("gnome-podcasts");

        // Make sure the app icon shows up in PulseAudio settings
        env::set_var("PULSE_PROP_application.icon_name", APP_ID);

        let application = Self::new();

        // Weird magic I copy-pasted that sets the Application Name in the Shell.
        glib::set_application_name(&i18n("Podcasts"));
        glib::set_prgname(Some("gnome-podcasts"));
        gtk::Window::set_default_icon_name(APP_ID);
        let args: Vec<String> = env::args().collect();
        ApplicationExtManual::run(&application, &args);
    }
}

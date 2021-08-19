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

use glib::clone;
use glib::prelude::*;
use glib::subclass::prelude::*;

use gio::subclass::prelude::ApplicationImplExt;
use gio::{self, prelude::*, ApplicationFlags};

use gtk::prelude::*;
use libhandy::prelude::*;

use gettextrs::{bindtextdomain, setlocale, textdomain, LocaleCategory};

use anyhow::Result;
use fragile::Fragile;
use podcasts_data::dbqueries;
use podcasts_data::{Episode, Show, Source};

use crate::settings;
use crate::stacks::PopulatedState;
use crate::utils;
use crate::widgets::appnotif::{InAppNotification, SpinnerState, State};
use crate::widgets::show_menu::{mark_all_notif, remove_show_notif};
use crate::widgets::EpisodeDescription;
use crate::window::MainWindow;

use std::cell::RefCell;
use std::env;
use std::sync::Arc;

use crate::config::{APP_ID, LOCALEDIR};
use crate::i18n::i18n;

// FIXME: port Optionals to OnceCell
#[derive(Debug)]
pub struct PdApplicationPrivate {
    sender: glib::Sender<Action>,
    receiver: RefCell<Option<glib::Receiver<Action>>>,
    window: RefCell<Option<MainWindow>>,
    settings: RefCell<Option<gio::Settings>>,
    inhibit_cookie: RefCell<u32>,
}

#[glib::object_subclass]
impl ObjectSubclass for PdApplicationPrivate {
    const NAME: &'static str = "PdApplication";
    type Type = PdApplication;
    type ParentType = gtk::Application;
    type Interfaces = ();

    fn new() -> Self {
        let (sender, r) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let receiver = RefCell::new(Some(r));

        Self {
            sender,
            receiver,
            window: RefCell::new(None),
            settings: RefCell::new(None),
            inhibit_cookie: RefCell::new(0),
        }
    }
}

impl ObjectImpl for PdApplicationPrivate {}

impl gio::subclass::prelude::ApplicationImpl for PdApplicationPrivate {
    fn activate(&self, app: &PdApplication) {
        debug!("GtkApplication<PdApplication>::activate");

        if let Some(ref window) = *self.window.borrow() {
            // Ideally Gtk4/GtkBuilder make this irrelvent
            window.show_all();
            window.present();
            info!("Window presented");
            return;
        }

        let app = app.clone().downcast::<PdApplication>().expect("How?");
        app.setup_gactions();

        let window = MainWindow::new(&app, &self.sender);
        window.setup_gactions();
        window.show_all();
        window.present();
        self.window.replace(Some(window));

        app.setup_accels();

        // Setup action channel
        let receiver = self.receiver.borrow_mut().take().unwrap();
        receiver.attach(None, move |action| app.do_action(action));
    }

    fn startup(&self, app: &PdApplication) {
        debug!("GtkApplication<PdApplication>::startup");

        self.parent_startup(app);

        let settings = gio::Settings::new(APP_ID);

        let cleanup_date = settings::get_cleanup_date(&settings);
        // Garbage collect watched episodes from the disk
        utils::cleanup(cleanup_date);

        self.settings.replace(Some(settings));

        let app = app.clone().downcast::<PdApplication>().expect("How?");
        app.setup_timed_callbacks();
    }
}

impl gtk::subclass::application::GtkApplicationImpl for PdApplicationPrivate {}

glib::wrapper! {
    pub struct PdApplication(ObjectSubclass<PdApplicationPrivate>) @extends gio::Application, gtk::Application;
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
    GoToEpisodeDescription(Arc<Show>, Arc<Episode>),
    HeaderBarShowTile(String),
    HeaderBarNormal,
    CopiedUrlNotification,
    MarkAllPlayerNotification(Arc<Show>),
    UpdateFeed(Option<Vec<Source>>),
    ShowUpdateNotif(crossbeam_channel::Receiver<bool>),
    StopUpdating,
    RemoveShow(Arc<Show>),
    ErrorNotification(String),
    InitEpisode(i32),
    InitSecondaryMenu(Fragile<gio::MenuModel>),
    MoveBackOnDeck,
    EmptyState,
    PopulatedState,
    RaiseWindow,
    InhibitSuspend,
    UninhibitSuspend,
}

impl PdApplication {
    pub(crate) fn new() -> Self {
        let application = glib::Object::new::<PdApplication>(&[
            ("application-id", &Some(APP_ID)),
            ("flags", &ApplicationFlags::empty()),
        ])
        .expect("Application initialization failed...");

        application.set_resource_base_path(Some("/org/gnome/Podcasts"));

        application
    }

    fn setup_timed_callbacks(&self) {
        self.setup_dark_theme();
    }

    fn setup_gactions(&self) {
        let app = self.upcast_ref::<gtk::Application>();
        // Create the quit action
        utils::make_action(
            app,
            "quit",
            clone!(@weak self as app => move |_, _| {
                    app.quit();
            }),
        );
        let i32_variant_type = i32::static_variant_type();
        let show_episode = gio::SimpleAction::new("go-to-episode", Some(&i32_variant_type));
        show_episode.connect_activate(clone!(@weak self as app => move |_, id_variant_option| {
            match app.go_to_episode(id_variant_option) {
                Ok(_) => (),
                Err(e) => eprintln!("failed action app.go-to-episode: {}", e)
            }
        }));
        app.add_action(&show_episode);
    }

    fn go_to_episode(&self, id_variant_option: Option<&glib::Variant>) -> Result<()> {
        let id_variant = id_variant_option.expect("missing action_target_value");
        let id = id_variant.get::<i32>().expect("invalid variant type");
        let ep = dbqueries::get_episode_from_rowid(id)?;
        let show = dbqueries::get_podcast_from_id(ep.show_id())?;
        let data = PdApplicationPrivate::from_instance(self);
        send!(
            data.sender,
            Action::GoToEpisodeDescription(Arc::new(show), Arc::new(ep))
        );
        Ok(())
    }

    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<primary>q"]);
        // Bind the hamburger menu button to `F10`
        self.set_accels_for_action("win.menu", &["F10"]);
        self.set_accels_for_action("win.refresh", &["<primary>r"]);
    }

    fn setup_dark_theme(&self) {
        let data = PdApplicationPrivate::from_instance(self);
        if let Some(ref settings) = *data.settings.borrow() {
            let gtk_settings = gtk::Settings::default().unwrap();
            settings
                .bind(
                    "dark-theme",
                    &gtk_settings,
                    "gtk-application-prefer-dark-theme",
                )
                .flags(gio::SettingsBindFlags::DEFAULT)
                .build();
        } else {
            debug_assert!(false, "Well how'd you manage that?");
        }
    }

    fn do_action(&self, action: Action) -> glib::Continue {
        let data = PdApplicationPrivate::from_instance(self);
        let w = data.window.borrow();
        let window = w.as_ref().expect("Window is not initialized");

        info!("Incoming channel action: {:?}", action);
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
                window
                    .main_deck
                    .navigate(libhandy::NavigationDirection::Back);
                let shows = window.content.get_shows();
                let pop = shows.borrow().populated();
                window
                    .content
                    .get_stack()
                    .set_visible_child_full("shows", gtk::StackTransitionType::SlideLeft);
                pop.borrow_mut()
                    .switch_visible(PopulatedState::Widget, gtk::StackTransitionType::SlideLeft);
            }
            Action::ShowShowsAnimated => {
                window
                    .main_deck
                    .navigate(libhandy::NavigationDirection::Back);
                let shows = window.content.get_shows();
                let pop = shows.borrow().populated();
                pop.borrow_mut()
                    .switch_visible(PopulatedState::View, gtk::StackTransitionType::SlideRight);
            }
            Action::MoveBackOnDeck => {
                window
                    .main_deck
                    .navigate(libhandy::NavigationDirection::Back);
                window.headerbar.reveal_bottom_switcher(true);
                window.headerbar.update_bottom_switcher();
            }
            Action::GoToEpisodeDescription(show, ep) => {
                window.clear_deck();
                let description_widget = EpisodeDescription::new(ep, show, window.sender.clone());
                description_widget.container.set_widget_name("description");
                window.main_deck.add(&description_widget.container);
                window
                    .main_deck
                    .navigate(libhandy::NavigationDirection::Forward);
                window.headerbar.reveal_bottom_switcher(false);
            }
            Action::HeaderBarShowTile(title) => window.headerbar.switch_to_back(&title),
            Action::HeaderBarNormal => window.headerbar.switch_to_normal(),
            Action::CopiedUrlNotification => {
                let notif = EpisodeDescription::copied_url_notif();
                notif.show(&window.overlay, &window.headerbar.container);
            }
            Action::MarkAllPlayerNotification(pd) => {
                let notif = mark_all_notif(pd, &data.sender);
                notif.show(&window.overlay, &window.headerbar.container);
            }
            Action::RemoveShow(pd) => {
                let notif = remove_show_notif(pd, data.sender.clone());
                notif.show(&window.overlay, &window.headerbar.container);
            }
            Action::ErrorNotification(err) => {
                error!("An error notification was triggered: {}", err);
                let callback = |revealer: gtk::Revealer| {
                    revealer.set_reveal_child(false);
                    glib::Continue(false)
                };
                let undo_cb: Option<fn()> = None;
                let notif = InAppNotification::new(&err, 6000, callback, undo_cb);
                notif.show(&window.overlay, &window.headerbar.container);
            }
            Action::UpdateFeed(source) => {
                if window.updating.get() {
                    info!("Ignoring feed update request (another one is already running)")
                } else {
                    window.updating.set(true);
                    utils::refresh_feed(source, data.sender.clone())
                }
            }
            Action::StopUpdating => {
                window.updating.set(false);
            }
            Action::ShowUpdateNotif(receiver) => {
                use crossbeam_channel::TryRecvError;

                let sender = data.sender.clone();
                let callback = move |revealer: gtk::Revealer| match receiver.try_recv() {
                    Err(TryRecvError::Empty) => glib::Continue(true),
                    Err(TryRecvError::Disconnected) => glib::Continue(false),
                    Ok(_) => {
                        revealer.set_reveal_child(false);

                        send!(sender, Action::StopUpdating);
                        send!(sender, Action::RefreshAllViews);

                        glib::Continue(false)
                    }
                };
                let txt = i18n("Fetching new episodes");
                let undo_cb: Option<fn()> = None;
                let updater = InAppNotification::new(&txt, 250, callback, undo_cb);
                updater.set_close_state(State::Hidden);
                updater.set_spinner_state(SpinnerState::Active);

                let old = window.updater.replace(Some(updater));
                if let Some(i) = old {
                    unsafe { i.destroy() }
                }

                if let Some(i) = window.updater.borrow().as_ref() {
                    i.show(&window.overlay, &window.headerbar.container)
                }
            }
            Action::InitEpisode(rowid) => {
                let res = window.player.borrow_mut().initialize_episode(rowid);
                debug_assert!(res.is_ok());
            }
            Action::InitSecondaryMenu(s) => {
                let menu = &s.get();
                window.headerbar.set_secondary_menu(menu);
            }
            Action::EmptyState => {
                if let Some(refresh_action) = window
                    .window
                    .lookup_action("refresh")
                    .and_then(|action| action.downcast::<gio::SimpleAction>().ok())
                {
                    refresh_action.set_enabled(false)
                }

                window.headerbar.switch.set_sensitive(false);
                window.content.switch_to_empty_views();
            }
            Action::PopulatedState => {
                if let Some(refresh_action) = window
                    .window
                    .lookup_action("refresh")
                    .and_then(|action| action.downcast::<gio::SimpleAction>().ok())
                {
                    refresh_action.set_enabled(true)
                }

                window.headerbar.switch.set_sensitive(true);
                window.content.switch_to_populated();
            }
            Action::RaiseWindow => window.window.present(),
            Action::InhibitSuspend => {
                let window: Option<&gtk::Window> = None;
                let old_cookie = *data.inhibit_cookie.borrow();
                let cookie = self.inhibit(
                    window,
                    gtk::ApplicationInhibitFlags::SUSPEND,
                    Some("podcast playing"),
                );
                *data.inhibit_cookie.borrow_mut() = cookie;
                if old_cookie != 0 {
                    self.uninhibit(old_cookie);
                }
            }
            Action::UninhibitSuspend => {
                let cookie = *data.inhibit_cookie.borrow();
                if cookie != 0 {
                    self.uninhibit(cookie);
                    *data.inhibit_cookie.borrow_mut() = 0;
                }
            }
        };

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
        ApplicationExtManual::run_with_args(&application, &args);
    }
}

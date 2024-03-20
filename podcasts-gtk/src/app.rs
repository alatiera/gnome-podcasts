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

use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

use gettextrs::{bindtextdomain, setlocale, textdomain, LocaleCategory};

use anyhow::Result;
use podcasts_data::dbqueries;
use podcasts_data::discovery::FoundPodcast;
use podcasts_data::pipeline::pipeline;
use podcasts_data::{Episode, Show, Source};

use crate::settings;
use crate::utils;
use crate::widgets::show_menu::{mark_all_notif, remove_show_notif};
use crate::widgets::{DiscoveryPage, EpisodeDescription, SearchResults};
use crate::window::MainWindow;

use std::cell::RefCell;
use std::collections::HashSet;
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
    todo_unsub_ids: RefCell<HashSet<i32>>,
    undo_marked_ids: RefCell<Vec<i32>>,
}

#[glib::object_subclass]
impl ObjectSubclass for PdApplicationPrivate {
    const NAME: &'static str = "PdApplication";
    type Type = PdApplication;
    type ParentType = adw::Application;

    fn new() -> Self {
        let (sender, r) = glib::MainContext::channel(glib::Priority::default());
        let receiver = RefCell::new(Some(r));

        Self {
            sender,
            receiver,
            window: RefCell::new(None),
            settings: RefCell::new(None),
            inhibit_cookie: RefCell::new(0),
            todo_unsub_ids: RefCell::new(HashSet::default()),
            undo_marked_ids: RefCell::new(vec![]),
        }
    }
}

impl ObjectImpl for PdApplicationPrivate {}

impl ApplicationImpl for PdApplicationPrivate {
    fn activate(&self) {
        debug!("GtkApplication<PdApplication>::activate");

        self.parent_activate();

        if let Some(ref window) = *self.window.borrow() {
            // Ideally Gtk4/GtkBuilder make this irrelvent
            window.present();
            info!("Window presented");
            return;
        }

        let app = self.obj();
        app.setup_gactions();

        let window = MainWindow::new(&app, &self.sender);
        window.present();
        self.window.replace(Some(window));

        app.setup_accels();

        // Setup action channel
        let receiver = self.receiver.take().unwrap();
        receiver.attach(
            None,
            clone!(@weak app => @default-panic, move |action| {
                app.do_action(action)
            }),
        );
    }

    fn startup(&self) {
        debug!("GtkApplication<PdApplication>::startup");

        self.parent_startup();

        let settings = gio::Settings::new(APP_ID);

        let cleanup_date = settings::get_cleanup_date(&settings);
        // Garbage collect watched episodes from the disk
        utils::cleanup(cleanup_date);

        self.settings.replace(Some(settings));
    }

    fn shutdown(&self) {
        // complete any pending unsubscribe actions
        let mut todo_unsub_ids = self.todo_unsub_ids.borrow_mut();
        for id in todo_unsub_ids.drain() {
            if let Ok(pd) = dbqueries::get_podcast_from_id(id) {
                if let Err(err) = podcasts_data::utils::delete_show(&pd) {
                    error!("Error: {}", err);
                    error!("Failed to delete {}", pd.title());
                }
            }
        }
        self.parent_shutdown();
    }
}

impl GtkApplicationImpl for PdApplicationPrivate {}
impl AdwApplicationImpl for PdApplicationPrivate {}

glib::wrapper! {
    pub struct PdApplication(ObjectSubclass<PdApplicationPrivate>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

#[derive(Debug, Clone)]
pub(crate) enum Action {
    RefreshAllViews,
    RefreshEpisodesView,
    RefreshEpisodesViewBGR,
    RefreshShowsView,
    ReplaceWidget(Arc<Show>),
    RefreshWidgetIfSame(i32),
    GoToEpisodeDescription(Arc<Show>, Arc<Episode>),
    GoToShow(Arc<Show>),
    GoToDiscovery,
    GoToFoundPodcasts(Arc<Vec<FoundPodcast>>),
    CopiedUrlNotification,
    MarkAllPlayerNotification(Arc<Show>),
    UpdateFeed(Option<Vec<Source>>),
    ShowUpdateNotif,
    FeedRefreshed,
    StopUpdating,
    RemoveShow(Arc<Show>),
    ErrorNotification(String),
    InitEpisode(i32),
    InitEpisodeAt(i32, i32),
    StreamEpisode(i32),
    EmptyState,
    PopulatedState,
    Subscribe(String),
    RaiseWindow,
    InhibitSuspend,
    UninhibitSuspend,
}

impl PdApplication {
    pub(crate) fn new() -> Self {
        glib::Object::builder()
            .property("application-id", APP_ID)
            .property("resource-base-path", "/org/gnome/Podcasts")
            .build()
    }

    fn setup_gactions(&self) {
        let i32_variant_type = glib::VariantTy::INT32;
        let actions = [
            gio::ActionEntryBuilder::new("quit")
                .activate(|app: &Self, _, _| {
                    app.quit();
                })
                .build(),
            gio::ActionEntryBuilder::new("go-to-episode")
                .parameter_type(Some(i32_variant_type))
                .activate(|app: &Self, _, id_variant_option| {
                    if let Err(e) = app.go_to_episode(id_variant_option) {
                        error!("failed action app.go-to-episode: {e}");
                    }
                })
                .build(),
            gio::ActionEntryBuilder::new("go-to-show")
                .parameter_type(Some(i32_variant_type))
                .activate(|app: &Self, _, id_variant_option| {
                    if let Err(e) = app.go_to_show(id_variant_option) {
                        error!("failed action app.go-to-show: {e}");
                    }
                })
                .build(),
            gio::ActionEntryBuilder::new("undo-mark-all")
                .parameter_type(Some(i32_variant_type))
                .activate(|app: &Self, _, id_variant_option| {
                    let data = app.imp();
                    let id = id_variant_option.unwrap().get::<i32>().unwrap();
                    let mut ids = data.undo_marked_ids.borrow_mut();
                    if !ids.contains(&id) {
                        ids.push(id);
                    }

                    send!(data.sender, Action::RefreshWidgetIfSame(id));
                })
                .build(),
            gio::ActionEntryBuilder::new("undo-remove-show")
                .parameter_type(Some(i32_variant_type))
                .activate(|app: &Self, _, id_variant_option| {
                    let data = app.imp();
                    let id = id_variant_option.unwrap().get::<i32>().unwrap();
                    let mut ids = data.todo_unsub_ids.borrow_mut();
                    ids.remove(&id);

                    let res = utils::unignore_show(id);
                    debug_assert!(res.is_ok());
                    send!(data.sender, Action::RefreshShowsView);
                    send!(data.sender, Action::RefreshEpisodesView);
                })
                .build(),
        ];
        self.add_action_entries(actions);
    }

    /// We check if a show is still on the todo_remove list.
    /// Also removes it from the list,
    /// as this should only be called before removal.
    pub fn is_show_marked_delete(&self, pd: &Show) -> bool {
        let data = self.imp();
        let id = pd.id();
        let mut todo_unsub_ids = data.todo_unsub_ids.borrow_mut();

        todo_unsub_ids.remove(&id)
    }

    pub fn is_show_marked_mark(&self, pd: &Show) -> bool {
        let data = self.imp();
        let id = pd.id();
        let mut undo_marked_ids = data.undo_marked_ids.borrow_mut();

        if let Some(pos) = undo_marked_ids.iter().position(|x| *x == id) {
            undo_marked_ids.remove(pos);

            return false;
        }

        true
    }

    fn go_to_episode(&self, id_variant_option: Option<&glib::Variant>) -> Result<()> {
        let id_variant = id_variant_option.expect("missing action_target_value");
        let id = id_variant.get::<i32>().expect("invalid variant type");
        let ep = dbqueries::get_episode_from_id(id)?;
        let show = dbqueries::get_podcast_from_id(ep.show_id())?;
        let data = self.imp();
        send!(
            data.sender,
            Action::GoToEpisodeDescription(Arc::new(show), Arc::new(ep))
        );
        Ok(())
    }

    fn go_to_show(&self, id_variant_option: Option<&glib::Variant>) -> Result<()> {
        let id_variant = id_variant_option.expect("missing action_target_value");
        let id = id_variant.get::<i32>().expect("invalid variant type");
        let show = dbqueries::get_podcast_from_id(id)?;
        let data = self.imp();
        send!(data.sender, Action::GoToShow(Arc::new(show)));
        Ok(())
    }

    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<primary>q"]);
        self.set_accels_for_action("win.refresh", &["<primary>r"]);
    }

    fn do_action(&self, action: Action) -> glib::ControlFlow {
        let data = self.imp();
        let w = data.window.borrow();
        let window = w.as_ref().expect("Window is not initialized");

        info!("Incoming channel action: {:?}", action);
        match action {
            Action::RefreshAllViews => window.content().update(),
            Action::RefreshShowsView => window.content().update_shows_view(),
            Action::RefreshWidgetIfSame(id) => window.content().update_widget_if_same(id),
            Action::RefreshEpisodesView => window.content().update_home(),
            Action::RefreshEpisodesViewBGR => window.content().update_home_if_background(),
            Action::ReplaceWidget(pd) => {
                let shows = window.content().get_shows();
                let pop = shows.borrow().populated();
                pop.borrow_mut()
                    .replace_widget(pd.clone())
                    .map_err(|err| error!("Failed to update ShowWidget: {}", err))
                    .map_err(|_| error!("Failed to update ShowWidget {}", pd.title()))
                    .ok();
            }
            Action::GoToEpisodeDescription(show, ep) => {
                let description_widget = EpisodeDescription::new(ep, show, window.sender().clone());
                window.push_page(&description_widget);
            }
            Action::GoToShow(pd) => {
                self.do_action(Action::ReplaceWidget(pd));
                window.go_to_show_widget();
            }
            Action::GoToDiscovery => {
                let widget = DiscoveryPage::new(window.sender());
                window.push_page(&widget);
            }
            Action::GoToFoundPodcasts(found) => {
                let widget = SearchResults::new(&found, window.sender());
                window.push_page(&widget);
            }
            Action::CopiedUrlNotification => {
                let text = i18n("Copied URL to clipboard!");
                let toast = adw::Toast::new(&text);
                self.send_toast(toast);
            }
            Action::MarkAllPlayerNotification(pd) => {
                let toast = mark_all_notif(pd, &data.sender);
                self.send_toast(toast);
            }
            Action::RemoveShow(pd) => {
                data.todo_unsub_ids.borrow_mut().insert(pd.id());
                let toast = remove_show_notif(pd, data.sender.clone());
                self.send_toast(toast);
            }
            Action::ErrorNotification(err) => {
                error!("An error notification was triggered: {}", err);
                let toast = adw::Toast::new(&err);
                window.add_toast(toast);
            }
            Action::UpdateFeed(source) => {
                if window.updating() {
                    info!("Ignoring feed update request (another one is already running)")
                } else {
                    window.set_updating(true);
                    utils::refresh_feed(source, data.sender.clone())
                }
            }
            Action::StopUpdating => {
                window.set_updating(false);
                window.set_updating_timeout(None);
                window.progress_bar().set_visible(false);
            }
            Action::ShowUpdateNotif => {
                let progress = window.progress_bar();
                let updating_timeout = glib::timeout_add_local(
                    std::time::Duration::from_millis(100),
                    clone!(@weak progress => @default-return glib::ControlFlow::Break, move || {
                        progress.set_visible(true);
                        progress.pulse();
                        glib::ControlFlow::Continue
                    }),
                );
                window.set_updating_timeout(Some(updating_timeout));
            }
            Action::FeedRefreshed => {
                let sender = data.sender.clone();
                send!(sender, Action::StopUpdating);
                send!(sender, Action::RefreshAllViews);
            }
            Action::InitEpisode(id) => {
                let res = window.init_episode(id, None, false);
                debug_assert!(res.is_ok());
            }
            Action::InitEpisodeAt(id, second) => {
                let res = window.init_episode(id, Some(second), false);
                debug_assert!(res.is_ok());
            }
            Action::StreamEpisode(id) => {
                let res = window.init_episode(id, None, true);
                debug_assert!(res.is_ok());
            }
            Action::EmptyState => {
                if let Some(refresh_action) = window
                    .lookup_action("refresh")
                    .and_then(|action| action.downcast::<gio::SimpleAction>().ok())
                {
                    refresh_action.set_enabled(false)
                }

                window.headerbar().switch.set_sensitive(false);
                window.content().switch_to_empty_views();
            }
            Action::PopulatedState => {
                if let Some(refresh_action) = window
                    .lookup_action("refresh")
                    .and_then(|action| action.downcast::<gio::SimpleAction>().ok())
                {
                    refresh_action.set_enabled(true)
                }

                window.headerbar().switch.set_sensitive(true);
                window.content().switch_to_populated();
            }
            Action::RaiseWindow => window.present(),
            Action::Subscribe(feed) => {
                let sender = data.sender.clone();
                crate::RUNTIME.spawn(async move {
                    let mut error_source = None; // <- auto unsub from this
                    if let Err(e) = async {
                        let source =
                            dbqueries::get_source_from_uri(&feed).or(Source::from_url(&feed))?;
                        error_source = Some(source.clone());
                        let source_id = source.id();
                        info!("Subscribing to {feed}");
                        pipeline(vec![source]).await?;
                        let show = dbqueries::get_podcast_from_source_id(source_id)?;
                        send!(sender, Action::RefreshAllViews);
                        send!(sender, Action::GoToShow(Arc::new(show.clone())));
                        Ok::<(), anyhow::Error>(())
                    }
                    .await
                    {
                        error!("Failed to subscribe: {feed} {e}");
                        // auto unsubscribe
                        if let Some(error_source) = error_source {
                            // only unsub if no Show was imported from the source.
                            if dbqueries::get_podcast_from_source_id(error_source.id()).is_err() {
                                if let Err(remove_err) = dbqueries::remove_source(&error_source) {
                                    error!("failed to remove failed source! {remove_err} {feed}");
                                } else {
                                    info!("auto removed source that failed to import {feed}");
                                }
                            }
                        }
                        // TODO show the actual error (like "content didn't start with rss feed"),
                        // but pipeline doesn't pass useful errors yet
                        send!(
                            sender,
                            Action::ErrorNotification(format!(
                                "Failed to subscribe to feed: {feed} "
                            ))
                        );
                    }
                });
            }
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

        glib::ControlFlow::Continue
    }

    pub(crate) fn run() -> glib::ExitCode {
        // Set up the textdomain for gettext
        setlocale(LocaleCategory::LcAll, "");
        bindtextdomain("gnome-podcasts", LOCALEDIR).expect("Unable to bind the text domain");
        textdomain("gnome-podcasts").expect("Unable to switch to the text domain");

        // Make sure the app icon shows up in PulseAudio settings
        env::set_var("PULSE_PROP_application.icon_name", APP_ID);

        let application = Self::new();

        // Weird magic I copy-pasted that sets the Application Name in the Shell.
        glib::set_application_name(&i18n("Podcasts"));
        gtk::Window::set_default_icon_name(APP_ID);
        let args: Vec<String> = env::args().collect();
        ApplicationExtManual::run_with_args(&application, &args)
    }

    pub(crate) fn send_toast(&self, toast: adw::Toast) {
        self.imp()
            .window
            .borrow()
            .as_ref()
            .unwrap()
            .add_toast(toast);
    }
}

#![allow(new_without_default)]

use gio::{
    ActionMapExt, ApplicationExt, ApplicationExtManual, ApplicationFlags, Settings, SettingsExt,
    SimpleAction, SimpleActionExt,
};
use glib;
use gtk;
use gtk::prelude::*;
use gtk::SettingsExt as GtkSettingsExt;

use crossbeam_channel::{unbounded, Sender};
use hammond_data::Podcast;

use headerbar::Header;
use settings::{self, WindowGeometry};
use stacks::{Content, PopulatedState};
use utils;
use widgets::appnotif::{InAppNotification, UndoState};
use widgets::player;
use widgets::{about_dialog, mark_all_notif, remove_show_notif};

use std::rc::Rc;
use std::sync::Arc;

/// Creates an action named $called in the action map $on with the handler $handle
macro_rules! action {
    ($on:expr, $called:expr, $handle:expr) => {{
        // Create a stateless, parameterless action
        let act = SimpleAction::new($called, None);
        // Connect the handler
        act.connect_activate($handle);
        // Add it to the map
        $on.add_action(&act);
        // Return the action
        act
    }};
}

#[derive(Debug, Clone)]
pub enum Action {
    RefreshAllViews,
    RefreshEpisodesView,
    RefreshEpisodesViewBGR,
    RefreshShowsView,
    ReplaceWidget(Arc<Podcast>),
    RefreshWidgetIfSame(i32),
    ShowWidgetAnimated,
    ShowShowsAnimated,
    HeaderBarShowTile(String),
    HeaderBarNormal,
    HeaderBarShowUpdateIndicator,
    HeaderBarHideUpdateIndicator,
    MarkAllPlayerNotification(Arc<Podcast>),
    RemoveShow(Arc<Podcast>),
    ErrorNotification(String),
    InitEpisode(i32),
}

#[derive(Debug)]
pub struct App {
    app_instance: gtk::Application,
    settings: Settings,
}

impl App {
    pub fn new() -> App {
        let settings = Settings::new("org.gnome.Hammond");
        let application = gtk::Application::new("org.gnome.Hammond", ApplicationFlags::empty())
            .expect("Application Initialization failed...");

        let cleanup_date = settings::get_cleanup_date(&settings);
        utils::cleanup(cleanup_date);

        application.connect_startup(clone!(settings => move |app| {
            let (sender, receiver) = unbounded();

            app.set_accels_for_action("win.refresh", &["<primary>r"]);
            app.set_accels_for_action("win.quit", &["<primary>q"]);
            // Bind the hamburger menu button to `F10`
            app.set_accels_for_action("win.menu", &["F10"]);

            app.connect_activate(clone!(sender, settings => move |app| {
                // Get the current window (if any)
                if let Some(window) = app.get_active_window() {
                    // Already open, just raise the window
                    window.present();
                } else {
                    // Time to open one!
                    // Create the main window
                    let window = gtk::ApplicationWindow::new(&app);
                    window.set_title("Hammond");

                    App::setup_gactions(&window, &sender);

                    window.connect_delete_event(clone!(app, settings => move |window, _| {
                        WindowGeometry::from_window(&window).write(&settings);
                        app.quit();
                        Inhibit(false)
                    }));


                    // Create a content instance
                    let content =
                        Rc::new(Content::new(sender.clone()).expect(
                            "Content Initialization failed."));

                    // Create the headerbar
                    let header = Rc::new(Header::new(&content, &window, &sender));

                    action!(window, "menu", clone!(header => move |_, _| {
                        header.open_menu();
                    }));

                    // Add the content main stack to the overlay.
                    let overlay = gtk::Overlay::new();
                    overlay.add(&content.get_stack());

                    let wrap = gtk::Box::new(gtk::Orientation::Vertical, 0);
                    // Add the overlay to the main Box
                    wrap.add(&overlay);

                    let player = player::PlayerWidget::new(&sender);
                    // Add the player to the main Box
                    wrap.add(&player.action_bar);

                    window.add(&wrap);

                    WindowGeometry::from_settings(&settings).apply(&window);

                    App::setup_timed_callbacks(&sender, &settings);

                    window.show_all();
                    window.activate();

                    gtk::timeout_add(25, clone!(sender, receiver => move || {
                        // Uses receiver, content, header, sender, overlay, playback
                        match receiver.try_recv() {
                            Some(Action::RefreshAllViews) => content.update(),
                            Some(Action::RefreshShowsView) => content.update_shows_view(),
                            Some(Action::RefreshWidgetIfSame(id)) =>
                                content.update_widget_if_same(id),
                            Some(Action::RefreshEpisodesView) => content.update_home(),
                            Some(Action::RefreshEpisodesViewBGR) =>
                                content.update_home_if_background(),
                            Some(Action::ReplaceWidget(pd)) => {
                                let shows = content.get_shows();
                                let mut pop = shows.borrow().populated();
                                pop.borrow_mut()
                                    .replace_widget(pd.clone())
                                    .map_err(|err| error!("Failed to update ShowWidget: {}", err))
                                    .map_err(|_|
                                        error!("Failed ot update ShowWidget {}", pd.title()))
                                    .ok();
                            }
                            Some(Action::ShowWidgetAnimated) => {
                                let shows = content.get_shows();
                                let mut pop = shows.borrow().populated();
                                pop.borrow_mut().switch_visible(
                                    PopulatedState::Widget,
                                    gtk::StackTransitionType::SlideLeft,
                                );
                            }
                            Some(Action::ShowShowsAnimated) => {
                                let shows = content.get_shows();
                                let mut pop = shows.borrow().populated();
                                pop.borrow_mut()
                                    .switch_visible(PopulatedState::View,
                                                    gtk::StackTransitionType::SlideRight);
                            }
                            Some(Action::HeaderBarShowTile(title)) =>
                                header.switch_to_back(&title),
                            Some(Action::HeaderBarNormal) => header.switch_to_normal(),
                            Some(Action::HeaderBarShowUpdateIndicator) =>
                                header.show_update_notification(),
                            Some(Action::HeaderBarHideUpdateIndicator) =>
                                header.hide_update_notification(),
                            Some(Action::MarkAllPlayerNotification(pd)) => {
                                let notif = mark_all_notif(pd, &sender);
                                notif.show(&overlay);
                            }
                            Some(Action::RemoveShow(pd)) => {
                                let notif = remove_show_notif(pd, sender.clone());
                                notif.show(&overlay);
                            }
                            Some(Action::ErrorNotification(err)) => {
                                error!("An error notification was triggered: {}", err);
                                let callback = || glib::Continue(false);
                                let notif = InAppNotification::new(&err, callback,
                                                                   || {}, UndoState::Hidden);
                                notif.show(&overlay);
                            },
                            Some(Action::InitEpisode(rowid)) => player.initialize_episode(rowid).unwrap(),
                            None => (),
                        }

                        Continue(true)
                    }));
                }
            }));
        }));

        App {
            app_instance: application,
            settings,
        }
    }

    fn setup_timed_callbacks(sender: &Sender<Action>, settings: &Settings) {
        App::setup_dark_theme(settings);
        App::setup_refresh_on_startup(&sender, settings);
        App::setup_auto_refresh(&sender, settings);
    }

    fn setup_dark_theme(settings: &Settings) {
        let gtk_settings = gtk::Settings::get_default().unwrap();
        let enabled = settings.get_boolean("dark-theme");

        gtk_settings.set_property_gtk_application_prefer_dark_theme(enabled);
    }

    fn setup_refresh_on_startup(sender: &Sender<Action>, settings: &Settings) {
        // Update the feeds right after the Application is initialized.
        let sender = sender.clone();
        if settings.get_boolean("refresh-on-startup") {
            info!("Refresh on startup.");
            // The ui loads async, after initialization
            // so we need to delay this a bit so it won't block
            // requests that will come from loading the gui on startup.
            gtk::timeout_add(1500, move || {
                let s: Option<Vec<_>> = None;
                utils::refresh(s, sender.clone());
                glib::Continue(false)
            });
        }
    }

    fn setup_auto_refresh(sender: &Sender<Action>, settings: &Settings) {
        let refresh_interval = settings::get_refresh_interval(&settings).num_seconds() as u32;

        info!("Auto-refresh every {:?} seconds.", refresh_interval);

        let sender = sender.clone();
        gtk::timeout_add_seconds(refresh_interval, move || {
            let s: Option<Vec<_>> = None;
            utils::refresh(s, sender.clone());

            glib::Continue(true)
        });
    }

    /// Define the `GAction`s.
    ///
    /// Used in menus and the keyboard shortcuts dialog.
    fn setup_gactions(win: &gtk::ApplicationWindow, sender: &Sender<Action>) {
        // Create the `refresh` action.
        //
        // This will trigger a refresh of all the shows in the database.
        action!(
            win,
            "refresh",
            clone!(sender => move |_, _| {
            gtk::idle_add(clone!(sender => move || {
                let s: Option<Vec<_>> = None;
                utils::refresh(s, sender.clone());
                glib::Continue(false)
            }));
        })
        );

        // Create the `OPML` import action
        action!(
            win,
            "import",
            clone!(sender, win => move |_, _| {
            utils::on_import_clicked(&win, &sender);
        })
        );

        // Create the action that shows a `gtk::AboutDialog`
        action!(
            win,
            "about",
            clone!(win => move |_, _| {
            about_dialog(&win);
        })
        );

        // Create the quit action
        action!(
            win,
            "quit",
            clone!(win => move |_, _| win.get_application().expect("Failed to get application").quit())
        );
    }

    pub fn run(self) {
        // Weird magic I copy-pasted that sets the Application Name in the Shell.
        glib::set_application_name("Hammond");
        glib::set_prgname(Some("Hammond"));
        // We need out own org.gnome.Hammon icon
        gtk::Window::set_default_icon_name("multimedia-player");
        ApplicationExtManual::run(&self.app_instance, &[]);
    }
}

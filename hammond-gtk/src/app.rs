#![allow(new_without_default)]

use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, Settings, SettingsExt};
use glib;
use gtk;
use gtk::prelude::*;
use gtk::SettingsExt as GtkSettingsExt;

use failure::Error;
use rayon;

use hammond_data::utils::delete_show;
use hammond_data::{Podcast, Source};

use appnotif::*;
use headerbar::Header;
use settings::WindowGeometry;
use stacks::Content;
use utils;
use widgets::mark_all_watched;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum Action {
    UpdateSources(Option<Source>),
    RefreshAllViews,
    RefreshEpisodesView,
    RefreshEpisodesViewBGR,
    RefreshShowsView,
    RefreshWidget,
    RefreshWidgetIfVis,
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
}

#[derive(Debug)]
pub struct App {
    app_instance: gtk::Application,
    window: gtk::Window,
    overlay: gtk::Overlay,
    header: Arc<Header>,
    content: Arc<Content>,
    receiver: Receiver<Action>,
    sender: Sender<Action>,
    settings: Settings,
}

impl App {
    pub fn new() -> App {
        let settings = Settings::new("org.gnome.Hammond");
        let application = gtk::Application::new("org.gnome.Hammond", ApplicationFlags::empty())
            .expect("Application Initialization failed...");

        // Weird magic I copy-pasted that sets the Application Name in the Shell.
        glib::set_application_name("Hammond");
        glib::set_prgname(Some("Hammond"));

        // Create the main window
        let window = gtk::Window::new(gtk::WindowType::Toplevel);

        window.set_title("Hammond");

        let app_clone = application.clone();
        let window_clone = window.clone();
        let settings_clone = settings.clone();
        window.connect_delete_event(move |_, _| {
            WindowGeometry::from_window(&window_clone).write(&settings_clone);
            app_clone.quit();
            Inhibit(false)
        });

        let (sender, receiver) = channel();

        // Create a content instance
        let content =
            Arc::new(Content::new(sender.clone()).expect("Content Initialization failed."));

        // Create the headerbar
        let header = Arc::new(Header::new(&content, &window, sender.clone()));

        // Add the content main stack to the overlay.
        let overlay = gtk::Overlay::new();
        overlay.add(&content.get_stack());

        // Add the overlay to the main window
        window.add(&overlay);

        App {
            app_instance: application,
            window,
            overlay,
            header,
            content,
            receiver,
            sender,
            settings,
        }
    }

    fn setup_timed_callbacks(&self) {
        self.setup_dark_theme();
        self.setup_refresh_on_startup();
        self.setup_auto_refresh();
    }

    fn setup_dark_theme(&self) {
        let settings = gtk::Settings::get_default().unwrap();
        let enabled = self.settings.get_boolean("dark-theme");

        settings.set_property_gtk_application_prefer_dark_theme(enabled);
    }

    fn setup_refresh_on_startup(&self) {
        // Update the feeds right after the Application is initialized.
        if self.settings.get_boolean("refresh-on-startup") {
            let cleanup_date = utils::get_cleanup_date(&self.settings);
            let sender = self.sender.clone();

            info!("Refresh on startup.");

            utils::cleanup(cleanup_date);

            gtk::idle_add(move || {
                utils::refresh(None, sender.clone());
                glib::Continue(false)
            });
        }
    }

    fn setup_auto_refresh(&self) {
        let refresh_interval = utils::get_refresh_interval(&self.settings).num_seconds() as u32;
        let sender = self.sender.clone();

        info!("Auto-refresh every {:?} seconds.", refresh_interval);

        gtk::timeout_add_seconds(refresh_interval, move || {
            utils::refresh(None, sender.clone());

            glib::Continue(true)
        });
    }

    pub fn run(self) {
        WindowGeometry::from_settings(&self.settings).apply(&self.window);

        let window = self.window.clone();

        self.app_instance.connect_startup(move |app| {
            build_ui(&window, app);
        });
        self.setup_timed_callbacks();

        let content = self.content.clone();
        let headerbar = self.header.clone();
        let sender = self.sender.clone();
        let overlay = self.overlay.clone();
        let receiver = self.receiver;
        gtk::idle_add(move || {
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(Action::UpdateSources(source)) => {
                    if let Some(s) = source {
                        utils::refresh(Some(vec![s]), sender.clone());
                    } else {
                        utils::refresh(None, sender.clone());
                    }
                }
                Ok(Action::RefreshAllViews) => content.update(),
                Ok(Action::RefreshShowsView) => content.update_shows_view(),
                Ok(Action::RefreshWidget) => content.update_widget(),
                Ok(Action::RefreshWidgetIfVis) => content.update_widget_if_visible(),
                Ok(Action::RefreshWidgetIfSame(id)) => content.update_widget_if_same(id),
                Ok(Action::RefreshEpisodesView) => content.update_episode_view(),
                Ok(Action::RefreshEpisodesViewBGR) => content.update_episode_view_if_baground(),
                Ok(Action::ReplaceWidget(pd)) => {
                    if let Err(err) = content.get_shows().replace_widget(pd) {
                        error!("Something went wrong while trying to update the ShowWidget.");
                        error!("Error: {}", err);
                    }
                }
                Ok(Action::ShowWidgetAnimated) => content.get_shows().switch_widget_animated(),
                Ok(Action::ShowShowsAnimated) => content.get_shows().switch_podcasts_animated(),
                Ok(Action::HeaderBarShowTile(title)) => headerbar.switch_to_back(&title),
                Ok(Action::HeaderBarNormal) => headerbar.switch_to_normal(),
                Ok(Action::HeaderBarShowUpdateIndicator) => headerbar.show_update_notification(),
                Ok(Action::HeaderBarHideUpdateIndicator) => headerbar.hide_update_notification(),
                Ok(Action::MarkAllPlayerNotification(pd)) => {
                    let id = pd.id();
                    let callback = clone!(sender => move || {
                        if let Err(err) = mark_all_watched(&pd, sender.clone()) {
                            error!("Something went horribly wrong with the notif callback: {}", err);
                        }
                        glib::Continue(false)
                    });

                    let undo_callback = clone!(sender => move || {
                        sender.send(Action::RefreshWidgetIfSame(id)).expect("Action channel blow up");
                    });

                    let text = "Marked all episodes as listened".into();
                    let notif = InAppNotification::new(text, callback, undo_callback);
                    notif.show(&overlay);
                }
                Ok(Action::RemoveShow(pd)) => {
                    let text = format!("Unsubscribed from {}", pd.title());

                    if let Err(err) = utils::ignore_show(pd.id()) {
                        error!("Could not insert {} to the ignore list.", pd.title());
                        error!("Error: {}", err);
                    }

                    let callback = clone!(pd => move || {
                        if let Err(err) = utils::uningore_show(pd.id()) {
                            error!("Could not remove {} from the ignore list.", pd.title());
                            error!("Error: {}", err);
                        }

                        // Spawn a thread so it won't block the ui.
                        rayon::spawn(clone!(pd => move || {
                            if let Err(err) = delete_show(&pd) {
                                error!("Something went wrong trying to remove {}", pd.title());
                                error!("Error: {}", err);
                            }
                        }));
                        glib::Continue(false)
                    });

                    let sender_ = sender.clone();
                    let undo_wrap = move || -> Result<(), Error> {
                        utils::uningore_show(pd.id())?;
                        sender_.send(Action::RefreshShowsView)?;
                        sender_.send(Action::RefreshEpisodesView)?;
                        Ok(())
                    };

                    let undo_callback = move || {
                        if let Err(err) = undo_wrap() {
                            error!("{}", err)
                        }
                    };

                    let notif = InAppNotification::new(text, callback, undo_callback);
                    notif.show(&overlay);
                }
                Err(_) => (),
            }

            Continue(true)
        });

        ApplicationExtManual::run(&self.app_instance, &[]);
    }
}

fn build_ui(window: &gtk::Window, app: &gtk::Application) {
    window.set_application(app);
    window.show_all();
    window.activate();
    app.connect_activate(move |_| ());
}

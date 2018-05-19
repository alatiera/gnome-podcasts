#![allow(new_without_default)]

use gio::{ApplicationExt, ApplicationExtManual, ApplicationFlags, Settings, SettingsExt, SimpleAction, SimpleActionExt, ActionMapExt};
use glib;
use gtk;
use gtk::prelude::*;
use gtk::SettingsExt as GtkSettingsExt;

use hammond_data::Podcast;

use appnotif::{InAppNotification, UndoState};
use headerbar::Header;
use settings::{self, WindowGeometry};
use stacks::{Content, PopulatedState};
use utils;
use widgets::{mark_all_notif, remove_show_notif};

use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

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
}

#[derive(Debug)]
pub struct App {
    app_instance: gtk::Application,
    window: gtk::Window,
    overlay: gtk::Overlay,
    header: Rc<Header>,
    content: Rc<Content>,
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

        let cleanup_date = settings::get_cleanup_date(&settings);
        utils::cleanup(cleanup_date);

        // Create the main window
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Hammond");

        // Ideally a lot more than actions would happen in startup & window
        // creation would be in activate
        application.connect_startup(clone!(window => move |app| {
            let about = SimpleAction::new("about", None);
            // Should investigate use of active_window here
            about.connect_activate(clone!(window => move |_, _| about_dialog(&window)));
            app.add_action(&about);

            let quit = SimpleAction::new("quit", None);
            quit.connect_activate(clone!(app => move |_, _| app.quit()));
            app.add_action(&quit);
        }));

        window.connect_delete_event(clone!(application, settings, window => move |_, _| {
            WindowGeometry::from_window(&window).write(&settings);
            application.quit();
            Inhibit(false)
        }));

        let (sender, receiver) = channel();

        // Create a content instance
        let content =
            Rc::new(Content::new(sender.clone()).expect("Content Initialization failed."));

        // Create the headerbar
        let header = Rc::new(Header::new(&content, &window, &sender));

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
            let sender = self.sender.clone();

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

    fn setup_auto_refresh(&self) {
        let refresh_interval = settings::get_refresh_interval(&self.settings).num_seconds() as u32;
        let sender = self.sender.clone();

        info!("Auto-refresh every {:?} seconds.", refresh_interval);

        gtk::timeout_add_seconds(refresh_interval, move || {
            let s: Option<Vec<_>> = None;
            utils::refresh(s, sender.clone());

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

        let content = self.content;
        let headerbar = self.header;
        let sender = self.sender;
        let overlay = self.overlay;
        let receiver = self.receiver;
        gtk::timeout_add(50, move || {
            match receiver.try_recv() {
                Ok(Action::RefreshAllViews) => content.update(),
                Ok(Action::RefreshShowsView) => content.update_shows_view(),
                Ok(Action::RefreshWidgetIfSame(id)) => content.update_widget_if_same(id),
                Ok(Action::RefreshEpisodesView) => content.update_home(),
                Ok(Action::RefreshEpisodesViewBGR) => content.update_home_if_background(),
                Ok(Action::ReplaceWidget(pd)) => {
                    let shows = content.get_shows();
                    let mut pop = shows.borrow().populated();
                    pop.borrow_mut()
                        .replace_widget(pd.clone())
                        .map_err(|err| error!("Failed to update ShowWidget: {}", err))
                        .map_err(|_| error!("Failed ot update ShowWidget {}", pd.title()))
                        .ok();
                }
                Ok(Action::ShowWidgetAnimated) => {
                    let shows = content.get_shows();
                    let mut pop = shows.borrow().populated();
                    pop.borrow_mut().switch_visible(
                        PopulatedState::Widget,
                        gtk::StackTransitionType::SlideLeft,
                    );
                }
                Ok(Action::ShowShowsAnimated) => {
                    let shows = content.get_shows();
                    let mut pop = shows.borrow().populated();
                    pop.borrow_mut()
                        .switch_visible(PopulatedState::View, gtk::StackTransitionType::SlideRight);
                }
                Ok(Action::HeaderBarShowTile(title)) => headerbar.switch_to_back(&title),
                Ok(Action::HeaderBarNormal) => headerbar.switch_to_normal(),
                Ok(Action::HeaderBarShowUpdateIndicator) => headerbar.show_update_notification(),
                Ok(Action::HeaderBarHideUpdateIndicator) => headerbar.hide_update_notification(),
                Ok(Action::MarkAllPlayerNotification(pd)) => {
                    let notif = mark_all_notif(pd, &sender);
                    notif.show(&overlay);
                }
                Ok(Action::RemoveShow(pd)) => {
                    let notif = remove_show_notif(pd, sender.clone());
                    notif.show(&overlay);
                }
                Ok(Action::ErrorNotification(err)) => {
                    error!("An error notification was triggered: {}", err);
                    let callback = || glib::Continue(false);
                    let notif = InAppNotification::new(&err, callback, || {}, UndoState::Hidden);
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

// Totally copied it from fractal.
// https://gitlab.gnome.org/danigm/fractal/blob/503e311e22b9d7540089d735b92af8e8f93560c5/fractal-gtk/src/app.rs#L1883-1912
fn about_dialog(window: &gtk::Window) {
    // Feel free to add yourself if you contribured.
    let authors = &[
        "Constantin Nickel",
        "Gabriele Musco",
        "James Wykeham-Martin",
        "Jordan Petridis",
        "Julian Sparber",
        "Rowan Lewis",
        "Zander Brown"
    ];

    let dialog = gtk::AboutDialog::new();
    // Waiting for a logo.
    // dialog.set_logo_icon_name("org.gnome.Hammond");
    dialog.set_logo_icon_name("multimedia-player");
    dialog.set_comments("Podcast Client for the GNOME Desktop.");
    dialog.set_copyright("© 2017, 2018 Jordan Petridis");
    dialog.set_license_type(gtk::License::Gpl30);
    dialog.set_modal(true);
    // TODO: make it show it fetches the commit hash from which it was built
    // and the version number is kept in sync automaticly
    dialog.set_version("0.3.3");
    dialog.set_program_name("Hammond");
    // TODO: Need a wiki page first.
    // dialog.set_website("https://wiki.gnome.org/Design/Apps/Potential/Podcasts");
    // dialog.set_website_label("Learn more about Hammond");
    dialog.set_transient_for(window);

    dialog.set_artists(&["Tobias Bernard"]);
    dialog.set_authors(authors);

    dialog.show();
}

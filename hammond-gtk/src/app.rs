use gtk;
use glib;
use gio;
use gtk::prelude::*;
use gio::{ActionMapExt, ApplicationExt, ApplicationExtManual, SimpleActionExt};

use hammond_data::utils::checkup;
use hammond_data::Source;

use headerbar::Header;
use content::Content;
use utils;

use std::sync::Arc;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Debug)]
pub enum Action {
    UpdateSources(Option<Source>),
    RefreshViews,
    RefreshEpisodesViewBGR,
    HeaderBarShowTile(String),
    HeaderBarNormal,
    HeaderBarHideUpdateIndicator,
}

#[derive(Debug)]
pub struct App {
    app_instance: gtk::Application,
    window: gtk::Window,
    header: Arc<Header>,
    content: Arc<Content>,
    receiver: Receiver<Action>,
    sender: Sender<Action>,
}

impl App {
    pub fn new() -> App {
        let application =
            gtk::Application::new("org.gnome.Hammond", gio::ApplicationFlags::empty())
                .expect("Initialization failed...");

        // Weird magic I copy-pasted that sets the Application Name in the Shell.
        glib::set_application_name("Hammond");
        glib::set_prgname(Some("Hammond"));

        // Create the main window
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_default_size(860, 640);
        window.set_title("Hammond");
        let app_clone = application.clone();
        window.connect_delete_event(move |_, _| {
            app_clone.quit();
            Inhibit(false)
        });

        let (sender, receiver) = channel();

        // Create a content instance
        let content = Content::new(sender.clone());

        // Create the headerbar
        let header = Header::new(content.clone(), sender.clone());

        // Add the Headerbar to the window.
        window.set_titlebar(&header.container);
        // Add the content main stack to the window.
        window.add(&content.get_stack());

        App {
            app_instance: application,
            window,
            header,
            content,
            receiver,
            sender,
        }
    }

    pub fn setup_actions(&self) {
        // Updates the database and refreshes every view.
        let update = gio::SimpleAction::new("update", None);
        let header = self.header.clone();
        let sender = self.sender.clone();
        update.connect_activate(move |_, _| {
            utils::refresh_feed(header.clone(), None, sender.clone());
        });
        self.app_instance.add_action(&update);
    }

    pub fn setup_timed_callbacks(&self) {
        let header = self.header.clone();
        let sender = self.sender.clone();
        // Update the feeds right after the Application is initialized.
        gtk::timeout_add_seconds(2, move || {
            utils::refresh_feed(header.clone(), None, sender.clone());
            glib::Continue(false)
        });

        let header = self.header.clone();
        let sender = self.sender.clone();
        // Auto-updater, runs every hour.
        // TODO: expose the interval in which it run to a user setting.
        // TODO: show notifications.
        gtk::timeout_add_seconds(3600, move || {
            utils::refresh_feed(header.clone(), None, sender.clone());
            glib::Continue(true)
        });

        // Run a database checkup once the application is initialized.
        gtk::timeout_add(300, || {
            let _ = checkup();
            glib::Continue(false)
        });
    }

    pub fn run(self) {
        let window = self.window.clone();
        let app = self.app_instance.clone();
        self.app_instance.connect_startup(move |_| {
            build_ui(&window, &app);
        });
        self.setup_timed_callbacks();
        self.setup_actions();

        let content = self.content.clone();
        let headerbar = self.header.clone();
        let sender = self.sender.clone();
        let receiver = self.receiver;
        gtk::timeout_add(250, move || {
            match receiver.try_recv() {
                Ok(Action::UpdateSources(source)) => {
                    if let Some(s) = source {
                        utils::refresh_feed(headerbar.clone(), Some(vec![s]), sender.clone())
                    }
                }
                Ok(Action::RefreshViews) => content.update(),
                Ok(Action::RefreshEpisodesViewBGR) => content.update_episode_view_if_baground(),
                Ok(Action::HeaderBarShowTile(title)) => headerbar.switch_to_back(&title),
                Ok(Action::HeaderBarNormal) => headerbar.switch_to_normal(),
                Ok(Action::HeaderBarHideUpdateIndicator) => headerbar.hide_update_notification(),
                _ => (),
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

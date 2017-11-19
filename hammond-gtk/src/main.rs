extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate glib;
extern crate gtk;

extern crate diesel;
extern crate dissolve;
extern crate hammond_data;
extern crate hammond_downloader;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate open;
// extern crate rayon;

// use rayon::prelude::*;
use log::LogLevel;
use hammond_data::utils::checkup;

use gtk::prelude::*;
use gio::{ActionMapExt, ApplicationExt, MenuExt, SimpleActionExt};

// http://gtk-rs.org/tuto/closures
#[macro_export]
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

mod views;
mod widgets;
mod headerbar;

mod utils;
mod static_resource;

use views::podcasts_view;

/*
THIS IS STILL A PROTOTYPE.
*/

fn build_ui(app: &gtk::Application) {
    let menu = gio::Menu::new();
    menu.append("Quit", "app.quit");
    menu.append("Checkup", "app.check");
    app.set_app_menu(&menu);

    // Get the main window
    let window = gtk::ApplicationWindow::new(app);
    window.set_default_size(1150, 650);
    // Setup the Stack that will manage the switch between podcasts_view and podcast_widget.
    let stack = podcasts_view::setup_stack();
    window.add(&stack);

    window.connect_delete_event(|w, _| {
        w.destroy();
        Inhibit(false)
    });

    // Setup quit in the app menu since default is overwritten.
    let quit = gio::SimpleAction::new("quit", None);
    let window2 = window.clone();
    quit.connect_activate(move |_, _| {
        window2.destroy();
    });
    app.add_action(&quit);

    // Setup the checkup in the app menu.
    let check = gio::SimpleAction::new("check", None);
    check.connect_activate(move |_, _| {
        let _ = checkup();
    });
    app.add_action(&check);

    // queue a db update 1 minute after the startup.
    gtk::idle_add(clone!(stack => move || {
        utils::refresh_feed(&stack, None, Some(60));
        glib::Continue(false)
    }));

    // Get the headerbar
    let header = headerbar::get_headerbar(&stack);
    window.set_titlebar(&header);

    window.show_all();
    window.activate();
    app.connect_activate(move |_| ());
}

fn main() {
    // TODO: make the the logger a cli -vv option
    loggerv::init_with_level(LogLevel::Info).unwrap();
    static_resource::init().expect("Something went wrong with the resource file initialization.");
    hammond_data::utils::init().expect("Hammond Initialazation failed.");

    let application = gtk::Application::new("org.gnome.Hammond", gio::ApplicationFlags::empty())
        .expect("Initialization failed...");

    application.connect_startup(move |app| {
        build_ui(app);
    });

    application.run(&[]);
}

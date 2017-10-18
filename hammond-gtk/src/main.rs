// extern crate glib;

extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate gtk;

extern crate diesel;
extern crate hammond_data;
extern crate hammond_downloader;
#[macro_use]
extern crate log;
extern crate loggerv;
extern crate open;

use log::LogLevel;
use diesel::prelude::*;
use hammond_data::dbqueries;
use hammond_data::index_feed;
use hammond_data::models::Episode;
use hammond_downloader::downloader;

use std::thread;
use std::sync::{Arc, Mutex};

use gtk::prelude::*;
use gio::ApplicationExt;
use gdk_pixbuf::Pixbuf;

/*
THIS IS STILL A PROTOTYPE.
THE CODE IS TERIBLE, SPAGHETTI AND HAS UNWRAPS EVERYWHERE.
*/

fn create_flowbox_child(title: &str, cover: Option<Pixbuf>) -> gtk::FlowBoxChild {
    let build_src = include_str!("../gtk/pd_fb_child.ui");
    let builder = gtk::Builder::new_from_string(build_src);

    // Copy of gnome-music AlbumWidget
    let box_: gtk::Box = builder.get_object("fb_child").unwrap();
    let pd_title: gtk::Label = builder.get_object("pd_title").unwrap();
    let pd_cover: gtk::Image = builder.get_object("pd_cover").unwrap();

    let events: gtk::EventBox = builder.get_object("events").unwrap();

    // GDK.TOUCH_MASK
    // https://developer.gnome.org/gdk3/stable/gdk3-Events.html#GDK-TOUCH-MASK:CAPS
    // http://gtk-rs.org/docs/gdk/constant.TOUCH_MASK.html
    events.add_events(4194304);

    pd_title.set_text(&title);

    if let Some(img) = cover {
        pd_cover.set_from_pixbuf(&img);
    };

    let fbc = gtk::FlowBoxChild::new();
    fbc.add(&box_);
    fbc
}

fn create_and_fill_list_store(
    connection: &SqliteConnection,
    builder: &gtk::Builder,
) -> gtk::ListStore {
    let podcast_model: gtk::ListStore = builder.get_object("PdListStore").unwrap();

    let podcasts = dbqueries::get_podcasts(connection).unwrap();

    for pd in &podcasts {
        podcast_model.insert_with_values(
            None,
            &[0, 1, 2, 3, 4],
            &[
                &pd.id(),
                &pd.title(),
                &pd.description(),
                &pd.link(),
                &pd.image_uri().unwrap_or_default(),
            ],
        );
    }

    podcast_model
}

fn podcast_widget(
    connection: Arc<Mutex<SqliteConnection>>,
    title: Option<&str>,
    description: Option<&str>,
    image: Option<Pixbuf>,
) -> gtk::Box {
    // Adapted from gnome-music AlbumWidget
    let pd_widget_source = include_str!("../gtk/podcast_widget.ui");
    let pd_widget_buidler = gtk::Builder::new_from_string(pd_widget_source);
    let pd_widget: gtk::Box = pd_widget_buidler.get_object("podcast_widget").unwrap();

    let cover: gtk::Image = pd_widget_buidler.get_object("cover").unwrap();
    let title_label: gtk::Label = pd_widget_buidler.get_object("title_label").unwrap();
    let desc_label: gtk::Label = pd_widget_buidler.get_object("description_label").unwrap();
    let view: gtk::Viewport = pd_widget_buidler.get_object("view").unwrap();

    if let Some(t) = title {
        title_label.set_text(t);
        let listbox = episodes_listbox(connection.clone(), t);
        view.add(&listbox);
    }

    if let Some(d) = description {
        desc_label.set_text(d);
    }

    if let Some(i) = image {
        cover.set_from_pixbuf(&i);
    }

    // (pd_widget, title_label, desc_label, cover)
    pd_widget
}

fn epidose_widget(
    connection: Arc<Mutex<SqliteConnection>>,
    episode: &mut Episode,
    pd_title: &str,
) -> gtk::Box {
    // This is just a prototype and will be reworked probably.
    let builder = include_str!("../gtk/EpisodeWidget.ui");
    let builder = gtk::Builder::new_from_string(builder);

    let ep: gtk::Box = builder.get_object("episode_box").unwrap();
    let dl_button: gtk::Button = builder.get_object("download_button").unwrap();
    let play_button: gtk::Button = builder.get_object("play_button").unwrap();

    let title_label: gtk::Label = builder.get_object("title_label").unwrap();
    let desc_label: gtk::Label = builder.get_object("desc_label").unwrap();

    title_label.set_xalign(0.0);
    desc_label.set_xalign(0.0);

    if let Some(t) = episode.title() {
        title_label.set_text(t);
    }

    if let Some(d) = episode.description() {
        desc_label.set_text(d);
    }

    if let Some(_) = episode.local_uri() {
        dl_button.hide();
        play_button.show();
        let uri = episode.local_uri().unwrap().to_owned();
        play_button.connect_clicked(move |_| {
            let e = open::that(&uri);
            if e.is_err() {
                error!("Error while trying to open: {}", uri);
            }
        });
    }

    let pd_title_cloned = pd_title.clone().to_owned();
    let db = connection.clone();
    let ep_clone = episode.clone();
    dl_button.connect_clicked(move |_| {
        // ugly hack to bypass the borrowchecker
        let pd_title = pd_title_cloned.clone();
        let db = db.clone();
        let mut ep_clone = ep_clone.clone();

        thread::spawn(move || {
            let dl_fold = downloader::get_dl_folder(&pd_title).unwrap();
            let tempdb = db.lock().unwrap();
            let e = downloader::get_episode(&tempdb, &mut ep_clone, dl_fold.as_str());
            if let Err(err) = e {
                error!("Error while trying to download: {}", ep_clone.uri());
                error!("Error: {}", err);
            };
        });
    });

    ep
}

fn episodes_listbox(connection: Arc<Mutex<SqliteConnection>>, pd_title: &str) -> gtk::ListBox {
    let m = connection.lock().unwrap();
    let pd = dbqueries::load_podcast(&m, pd_title).unwrap();
    let mut episodes = dbqueries::get_pd_episodes(&m, &pd).unwrap();
    drop(m);

    let list = gtk::ListBox::new();
    episodes.iter_mut().for_each(|ep| {
        let w = epidose_widget(connection.clone(), ep, pd_title);
        list.add(&w)
    });

    list.set_vexpand(false);
    list.set_hexpand(false);
    list.set_visible(true);
    list
}

fn refresh_db(db: Arc<Mutex<SqliteConnection>>) {
    let db_clone = db.clone();
    thread::spawn(move || {
        hammond_data::index_feed::index_loop(db_clone.clone(), false).unwrap();
    });
}

// I am sorry about the spaghetti code.
// Gonna clean it up when the GUI is a bit usuable.
fn build_ui() {
    let glade_src = include_str!("../gtk/foo.ui");
    let header_src = include_str!("../gtk/headerbar.ui");
    let builder = gtk::Builder::new_from_string(glade_src);
    let header_build = gtk::Builder::new_from_string(header_src);

    // Get the main window
    let window: gtk::Window = builder.get_object("window1").unwrap();
    // Get the Stack
    let stack: gtk::Stack = builder.get_object("stack1").unwrap();

    let db = Arc::new(Mutex::new(hammond_data::establish_connection()));
    let pd_widget = podcast_widget(db.clone(), None, None, None);
    stack.add_named(&pd_widget, "pdw");
    // Get the headerbar
    let header: gtk::HeaderBar = header_build.get_object("headerbar1").unwrap();
    window.set_titlebar(&header);

    // FIXME:
    // GLib-GIO-WARNING **: Your application does not implement g_application_activate()
    // and has no handlers connected to the 'activate' signal.  It should do one of these.
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // Adapted copy of the way gnome-music does albumview
    // FIXME: flowbox childs activate with space/enter but not with clicks.
    let flowbox: gtk::FlowBox = builder.get_object("flowbox1").unwrap();
    let grid: gtk::Grid = builder.get_object("grid").unwrap();

    // Stolen from gnome-news:
    // https://github.com/GNOME/gnome-news/blob/master/data/ui/headerbar.ui
    let add_toggle_button: gtk::MenuButton = header_build.get_object("add-toggle-button").unwrap();
    let add_popover: gtk::Popover = header_build.get_object("add-popover").unwrap();
    let new_url: gtk::Entry = header_build.get_object("new-url").unwrap();
    let add_button: gtk::Button = header_build.get_object("add-button").unwrap();
    // TODO: check if url exists in the db and lock the button
    new_url.connect_changed(move |url| {
        println!("{:?}", url.get_text());
    });
    let add_popover_clone = add_popover.clone();
    let db_clone = db.clone();
    add_button.connect_clicked(move |_| {
        let tempdb = db_clone.lock().unwrap();
        let url = new_url.get_text().unwrap();
        let _ = index_feed::insert_return_source(&tempdb, &url);
        drop(tempdb);
        println!("{:?} feed added", url);

        // update the db
        refresh_db(db_clone.clone());

        // TODO: lock the button instead of hiding and add notification of feed added.
        add_popover_clone.hide();
    });
    add_popover.hide();
    add_toggle_button.set_popover(&add_popover);

    let _search_button: gtk::Button = header_build.get_object("searchbutton").unwrap();

    // TODO: make it a back arrow button, that will hide when appropriate,
    // and add a StackSwitcher when more views are added.
    let home_button: gtk::Button = header_build.get_object("homebutton").unwrap();
    let grid_clone = grid.clone();
    let stack_clone = stack.clone();
    home_button.connect_clicked(move |_| stack_clone.set_visible_child(&grid_clone));

    let refresh_button: gtk::Button = header_build.get_object("refbutton").unwrap();
    // FIXME: There appears to be a memmory leak here.
    let db_clone = db.clone();
    refresh_button.connect_clicked(move |_| {
        // fsdaa, The things I do for the borrow checker.
        refresh_db(db_clone.clone());
    });

    let tempdb = db.lock().unwrap();
    let pd_model = create_and_fill_list_store(&tempdb, &builder);
    drop(tempdb);

    let iter = pd_model.get_iter_first().unwrap();
    // this will iterate over the episodes.
    // let iter = pd_model.iter_children(&iter).unwrap();
    loop {
        let title = pd_model.get_value(&iter, 1).get::<String>().unwrap();
        let description = pd_model.get_value(&iter, 2).get::<String>().unwrap();
        let image_uri = pd_model.get_value(&iter, 4).get::<String>();

        let imgpath = downloader::cache_image(&title, image_uri.as_ref().map(|s| s.as_str()));

        let pixbuf = if let Some(i) = imgpath {
            Pixbuf::new_from_file_at_scale(&i, 200, 200, true).ok()
        } else {
            None
        };

        let f = create_flowbox_child(&title, pixbuf.clone());
        let stack_clone = stack.clone();
        let db_clone = db.clone();
        f.connect_activate(move |_| {
            let pdw = stack_clone.get_child_by_name("pdw").unwrap();
            stack_clone.remove(&pdw);
            let pdw = podcast_widget(
                db_clone.clone(),
                Some(title.as_str()),
                Some(description.as_str()),
                pixbuf.clone(),
            );
            stack_clone.add_named(&pdw, "pdw");
            stack_clone.set_visible_child(&pdw);
            println!("Hello World!, child activated");
        });
        flowbox.add(&f);

        if !pd_model.iter_next(&iter) {
            break;
        }
    }

    window.show_all();
    gtk::main();
}

// Copied from:
// https://github.com/GuillaumeGomez/process-viewer/blob/ \
// ddcb30d01631c0083710cf486caf04c831d38cb7/src/process_viewer.rs#L367
fn main() {
    loggerv::init_with_level(LogLevel::Info).unwrap();
    hammond_data::init().unwrap();

    // Not sure if needed.
    if gtk::init().is_err() {
        info!("Failed to initialize GTK.");
        return;
    }

    let application = gtk::Application::new(
        "com.gitlab.alatiera.Hammond",
        gio::ApplicationFlags::empty(),
    ).expect("Initialization failed...");

    application.connect_startup(move |_| {
        build_ui();
    });

    // Not sure if this will be kept.
    let original = ::std::env::args().collect::<Vec<_>>();
    let mut tmp = Vec::with_capacity(original.len());
    for i in 0..original.len() {
        tmp.push(original[i].as_str());
    }
    application.run(&tmp);
}

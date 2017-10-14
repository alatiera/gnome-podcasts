// extern crate glib;
extern crate diesel;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate gtk;
extern crate hammond_data;
extern crate hammond_downloader;
#[macro_use]
extern crate log;
extern crate loggerv;

use log::LogLevel;
use diesel::prelude::*;
use hammond_data::dbqueries;

use gtk::TreeStore;
use gtk::prelude::*;
use gdk_pixbuf::Pixbuf;

fn create_flowbox_child(title: &str, image_uri: Option<&str>) -> gtk::FlowBoxChild {
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

    let imgpath = hammond_downloader::downloader::cache_image(title, image_uri);

    if let Some(i) = imgpath {
        let pixbuf = Pixbuf::new_from_file_at_scale(&i, 200, 200, true);
        if pixbuf.is_ok() {
            pd_cover.set_from_pixbuf(&pixbuf.unwrap())
        }
    };

    let fbc = gtk::FlowBoxChild::new();
    fbc.add(&box_);
    fbc
}

fn create_and_fill_tree_store(connection: &SqliteConnection, builder: &gtk::Builder) -> TreeStore {
    let podcast_model: TreeStore = builder.get_object("FooStore").unwrap();

    let podcasts = dbqueries::get_podcasts(connection).unwrap();

    for pd in &podcasts {
        let iter = podcast_model.insert_with_values(
            None,
            None,
            &[0, 1, 2, 3, 5],
            &[
                &pd.id(),
                &pd.title(),
                &pd.description(),
                &pd.link(),
                &pd.image_uri().unwrap_or_default(),
            ],
        );
        let episodes = dbqueries::get_pd_episodes(connection, &pd).unwrap();

        for ep in episodes {
            podcast_model.insert_with_values(
                Some(&iter),
                None,
                &[0, 1, 2, 6, 7, 8],
                &[
                    &ep.id(),
                    &ep.title().unwrap(),
                    &ep.description().unwrap_or_default(),
                    &ep.uri(),
                    &ep.local_uri().unwrap_or_default(),
                    &ep.published_date().unwrap_or_default(),
                ],
            );
        }
    }

    podcast_model
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

fn main() {
    loggerv::init_with_level(LogLevel::Info).unwrap();

    if gtk::init().is_err() {
        info!("Failed to initialize GTK.");
        return;
    }
    hammond_data::init().unwrap();

    let glade_src = include_str!("../gtk/foo.ui");
    // Adapted from gnome-music AlbumWidget
    let pd_widget = include_str!("../gtk/podcast_widget.ui");
    let header_src = include_str!("../gtk/headerbar.ui");
    let builder = gtk::Builder::new_from_string(glade_src);
    let header_build = gtk::Builder::new_from_string(header_src);
    let pd_widget = gtk::Builder::new_from_string(pd_widget);

    // Get the main window
    let window: gtk::Window = builder.get_object("window1").unwrap();
    // Get the Stack
    let stack: gtk::Stack = builder.get_object("stack1").unwrap();
    let pd_widget: gtk::Box = pd_widget.get_object("podcast_widget").unwrap();
    stack.add(&pd_widget);
    // Get the headerbar
    let header: gtk::HeaderBar = header_build.get_object("headerbar1").unwrap();
    window.set_titlebar(&header);

    let refresh_button: gtk::Button = header_build.get_object("refbutton").unwrap();
    // TODO: Have a small dropdown menu
    let _add_button: gtk::Button = header_build.get_object("addbutton").unwrap();
    let _search_button: gtk::Button = header_build.get_object("searchbutton").unwrap();
    let _home_button: gtk::Button = header_build.get_object("homebutton").unwrap();

    // FIXME: This locks the ui atm.
    refresh_button.connect_clicked(|_| {
        let db = hammond_data::establish_connection();
        hammond_data::index_feed::index_loop(db, false).unwrap();
    });

    // Exit cleanly on delete event
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    // Adapted copy of the way gnome-music does albumview
    let flowbox: gtk::FlowBox = builder.get_object("flowbox1").unwrap();

    let db = hammond_data::establish_connection();
    // let pd_model = create_and_fill_tree_store(&db, &builder);
    let pd_model = create_and_fill_list_store(&db, &builder);

    let iter = pd_model.get_iter_first().unwrap();
    // this will iterate over the episodes.
    // let iter = pd_model.iter_children(&iter).unwrap();
    loop {
        let title = pd_model.get_value(&iter, 1).get::<String>().unwrap();
        let image_uri = pd_model.get_value(&iter, 4).get::<String>();

        let f = create_flowbox_child(&title, image_uri.as_ref().map(|s| s.as_str()));
        let stack_clone = stack.clone();
        let pd_widget_clone = pd_widget.clone();
        f.connect_activate(move |_| {
            stack_clone.set_visible_child(&pd_widget_clone);
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

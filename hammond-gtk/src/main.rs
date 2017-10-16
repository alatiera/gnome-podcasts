// extern crate glib;
extern crate diesel;

extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate gtk;

extern crate hammond_data;
extern crate hammond_downloader;
#[macro_use]
extern crate log;
extern crate loggerv;

use log::LogLevel;
use diesel::prelude::*;
use hammond_data::dbqueries;

use gtk::prelude::*;
use gio::ApplicationExt;
use gdk_pixbuf::Pixbuf;

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

    if let Some(t) = title {
        title_label.set_text(t);
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

fn build_ui() {
    let glade_src = include_str!("../gtk/foo.ui");
    let header_src = include_str!("../gtk/headerbar.ui");
    let builder = gtk::Builder::new_from_string(glade_src);
    let header_build = gtk::Builder::new_from_string(header_src);

    // Get the main window
    let window: gtk::Window = builder.get_object("window1").unwrap();
    // Get the Stack
    let stack: gtk::Stack = builder.get_object("stack1").unwrap();

    let pd_widget = podcast_widget(None, None, None);
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
    new_url.connect_changed(move |url| {
        println!("{:?}", url.get_text());
    });
    // FIXME: Button is not clickable for some reason
    add_button.connect_clicked(move |f| {
        println!("{:?} feed added", f);
    });
    // add_button.clicked();
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
    // FIXME: This locks the ui atm.
    // FIXME: it also leaks memmory.
    refresh_button.connect_clicked(move |_| {
        let db = hammond_data::establish_connection();
        hammond_data::index_feed::index_loop(db, false).unwrap();
    });

    let db = hammond_data::establish_connection();
    // let pd_model = create_and_fill_tree_store(&db, &builder);
    let pd_model = create_and_fill_list_store(&db, &builder);

    let iter = pd_model.get_iter_first().unwrap();
    // this will iterate over the episodes.
    // let iter = pd_model.iter_children(&iter).unwrap();
    loop {
        let title = pd_model.get_value(&iter, 1).get::<String>().unwrap();
        let description = pd_model.get_value(&iter, 2).get::<String>().unwrap();
        let image_uri = pd_model.get_value(&iter, 4).get::<String>();

        let imgpath = hammond_downloader::downloader::cache_image(
            &title,
            image_uri.as_ref().map(|s| s.as_str()),
        );

        let pixbuf = if let Some(i) = imgpath {
            Pixbuf::new_from_file_at_scale(&i, 200, 200, true).ok()
        } else {
            None
        };

        let f = create_flowbox_child(&title, pixbuf.clone());
        let stack_clone = stack.clone();
        f.connect_activate(move |_| {
            let pdw = stack_clone.get_child_by_name("pdw").unwrap();
            stack_clone.remove(&pdw);
            let pdw = podcast_widget(
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

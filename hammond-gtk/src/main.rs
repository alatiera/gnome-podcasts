// extern crate glib;
extern crate diesel;
extern crate gtk;
// extern crate gdk_pixbuf;
extern crate hammond_data;

use diesel::prelude::*;
use gtk::prelude::*;
use gtk::Orientation;
use gtk::IconSize;
use gtk::Type;
use gtk::{CellRendererText, TreeStore, TreeView, TreeViewColumn};

use hammond_data::dbqueries;

use gtk::prelude::*;

// TODO: setup a img downloader, caching system, and then display them.
fn create_child(name: &str) -> gtk::Box {
    let box_ = gtk::Box::new(Orientation::Vertical, 5);
    let img = gtk::Image::new_from_icon_name("gtk-missing-image", IconSize::Menu.into());
    let label = gtk::Label::new(name);
    box_.set_size_request(200, 200);
    box_.pack_start(&img, true, true, 0);
    box_.pack_start(&label, false, false, 0);
    box_
}

fn create_tree_store(connection: &SqliteConnection, builder: &gtk::Builder) -> TreeStore {
    // let podcast_model = TreeStore::new(&[Type::String, Type::String,
    // Type::String]);
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
                    &ep.id(), &ep.title().unwrap(), &ep.description().unwrap_or_default(), &ep.uri(),
                    &ep.local_uri().unwrap_or_default(), &ep.published_date().unwrap_or_default(),
                ],
            );
        }
    }

    podcast_model
}

fn create_and_setup_view() -> TreeView {
    // Creating the tree view.
    let tree = TreeView::new();

    tree.set_headers_visible(false);

    // Creating the two columns inside the view.
    let column = TreeViewColumn::new();
    let cell = CellRendererText::new();

    column.pack_start(&cell, true);
    // Association of the view's column with the model's `id` column.
    column.add_attribute(&cell, "text", 1);
    tree.append_column(&column);

    let column = TreeViewColumn::new();
    let cell = CellRendererText::new();

    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", 2);
    tree.append_column(&column);

    let column = TreeViewColumn::new();
    let cell = CellRendererText::new();

    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", 3);
    tree.append_column(&column);

    tree
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }
    hammond_data::init().unwrap();

    // Adapted copy of the way gnome-music does albumview
    let glade_src = include_str!("../gtk/foo.ui");
    let builder = gtk::Builder::new_from_string(glade_src);

    // Get the main window
    // let window: gtk::Window = builder.get_object("window1").unwrap();
    let window: gtk::Window = builder.get_object("window2").unwrap();
    // Get the headerbar
    let header: gtk::HeaderBar = builder.get_object("headerbar1").unwrap();
    window.set_titlebar(&header);

    let refresh_button: gtk::Button = builder.get_object("refbutton").unwrap();
    // TODO: Have a small dropdown menu
    let _add_button: gtk::Button = builder.get_object("addbutton").unwrap();
    let _search_button: gtk::Button = builder.get_object("searchbutton").unwrap();
    let _home_button: gtk::Button = builder.get_object("homebutton").unwrap();

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

    // let flowbox: gtk::FlowBox = builder.get_object("flowbox1").unwrap();
    let db = hammond_data::establish_connection();
    let pd_model = create_tree_store(&db, &builder);
    // let podcasts = dbqueries::get_podcasts(&db).unwrap();

    // for pd in &podcasts {
    //     // TODO: This should be in a TreeStore.
    //     let f = create_child(pd.title());
    //     flowbox.add(&f);
    // }
    let box2: gtk::Box = builder.get_object("box2").unwrap();

    let treeview = create_and_setup_view();
    treeview.set_model(Some(&pd_model));
    box2.add(&treeview);

    window.show_all();
    gtk::main();
}

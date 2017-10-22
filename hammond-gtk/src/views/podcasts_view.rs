#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]

use gtk;
use gtk::prelude::*;

use diesel::prelude::SqliteConnection;
use hammond_data::dbqueries;

use std::sync::{Arc, Mutex};

use widgets::podcast::*;

// NOT IN USE.
// TRYING OUT STORELESS ATM.
pub fn populate_podcasts_flowbox(
    db: &Arc<Mutex<SqliteConnection>>,
    stack: &gtk::Stack,
    flowbox: &gtk::FlowBox,
) {
    let tempdb = db.lock().unwrap();
    let pd_model = podcast_liststore(&tempdb);
    drop(tempdb);

    // Get a ListStore iterator at the first element.
    let iter = if let Some(it) = pd_model.get_iter_first() {
        it
    } else {
        // stolen from gnome-news.
        let builder = include_str!("../../gtk/empty_view.ui");
        let builder = gtk::Builder::new_from_string(builder);
        let view: gtk::Box = builder.get_object("empty_view").unwrap();
        stack.add_named(&view, "empty");
        stack.set_visible_child_name("empty");

        info!("Empty view.");
        return;
    };

    loop {
        let title = pd_model
            .get_value(&iter, 1)
            .get::<String>()
            .unwrap_or_default();
        let description = pd_model.get_value(&iter, 2).get::<String>();
        let image_uri = pd_model.get_value(&iter, 4).get::<String>();

        let pixbuf = get_pixbuf_from_path(image_uri.as_ref().map(|s| s.as_str()), &title);
        let f = create_flowbox_child(&title, pixbuf.clone());

        let stack_clone = stack.clone();
        let db_clone = db.clone();

        f.connect_activate(move |_| {
            let old = stack_clone.get_child_by_name("pdw").unwrap();
            let pdw = podcast_widget(
                &db_clone,
                Some(title.as_str()),
                description.as_ref().map(|x| x.as_str()),
                pixbuf.clone(),
            );

            stack_clone.remove(&old);
            stack_clone.add_named(&pdw, "pdw");
            stack_clone.set_visible_child(&pdw);
            old.destroy();
            println!("Hello World!, child activated");
        });
        flowbox.add(&f);

        if !pd_model.iter_next(&iter) {
            break;
        }
    }
    flowbox.show_all();
}

fn show_empty_view(stack: &gtk::Stack) {
    let builder = include_str!("../../gtk/empty_view.ui");
    let builder = gtk::Builder::new_from_string(builder);
    let view: gtk::Box = builder.get_object("empty_view").unwrap();
    stack.add_named(&view, "empty");
    stack.set_visible_child_name("empty");

    info!("Empty view.");
}

pub fn pop_flowbox_no_store(
    db: &Arc<Mutex<SqliteConnection>>,
    stack: &gtk::Stack,
    flowbox: &gtk::FlowBox,
) {
    let podcasts = {
        let db = db.lock().unwrap();
        dbqueries::get_podcasts(&db)
    };

    if let Ok(pds) = podcasts {
        pds.iter().for_each(|parent| {
            let title = parent.title();
            let img = parent.image_uri();
            let pixbuf = get_pixbuf_from_path(img, title);
            let f = create_flowbox_child(title, pixbuf.clone());

            let db = db.clone();
            let stack = stack.clone();
            let parent = parent.clone();
            f.connect_activate(move |_| {
                on_flowbox_child_activate(&db, &stack, &parent, pixbuf.clone());
            });
            flowbox.add(&f);
        });
    } else {
        show_empty_view(stack);
    }
}

fn setup_podcast_widget(db: &Arc<Mutex<SqliteConnection>>, stack: &gtk::Stack) {
    let pd_widget = podcast_widget(db, None, None, None);
    stack.add_named(&pd_widget, "pdw");
}

fn setup_podcasts_grid(db: &Arc<Mutex<SqliteConnection>>, stack: &gtk::Stack) {
    let builder = include_str!("../../gtk/podcasts_view.ui");
    let builder = gtk::Builder::new_from_string(builder);
    let grid: gtk::Grid = builder.get_object("grid").unwrap();
    stack.add_named(&grid, "pd_grid");
    stack.set_visible_child(&grid);

    // Adapted copy of the way gnome-music does albumview
    // FIXME: flowbox childs activate with space/enter but not with clicks.
    let flowbox: gtk::FlowBox = builder.get_object("flowbox").unwrap();
    // Populate the flowbox with the Podcasts.
    // populate_podcasts_flowbox(db, stack, &flowbox);
    pop_flowbox_no_store(db, stack, &flowbox);
}

pub fn setup_stack(db: &Arc<Mutex<SqliteConnection>>) -> gtk::Stack {
    let stack = gtk::Stack::new();
    setup_podcast_widget(db, &stack);
    setup_podcasts_grid(db, &stack);
    stack
}

pub fn update_podcasts_view(db: &Arc<Mutex<SqliteConnection>>, stack: &gtk::Stack) {
    let builder = include_str!("../../gtk/podcasts_view.ui");
    let builder = gtk::Builder::new_from_string(builder);
    let grid: gtk::Grid = builder.get_object("grid").unwrap();

    let flowbox: gtk::FlowBox = builder.get_object("flowbox").unwrap();
    // Populate the flowbox with the Podcasts.
    populate_podcasts_flowbox(db, stack, &flowbox);

    let old = stack.get_child_by_name("pd_grid").unwrap();
    let vis = stack.get_visible_child_name().unwrap();

    stack.remove(&old);
    stack.add_named(&grid, "pd_grid");
    // preserve the visible widget
    stack.set_visible_child_name(&vis);
    old.destroy();
}

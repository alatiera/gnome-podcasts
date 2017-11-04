use gtk;
use gtk::prelude::*;

use hammond_data::dbqueries;
use hammond_data::index_feed::Database;

use widgets::podcast::*;

fn show_empty_view(stack: &gtk::Stack) {
    let builder = gtk::Builder::new_from_string(include_str!("../../gtk/empty_view.ui"));
    let view: gtk::Box = builder.get_object("empty_view").unwrap();
    stack.add_named(&view, "empty");
    stack.set_visible_child_name("empty");

    info!("Empty view.");
}

fn populate_flowbox(db: &Database, stack: &gtk::Stack, flowbox: &gtk::FlowBox) {
    let podcasts = {
        let db = db.lock().unwrap();
        dbqueries::get_podcasts(&db)
    };

    if let Ok(pds) = podcasts {
        pds.iter().for_each(|parent| {
            let f = create_flowbox_child(db, parent);
            flowbox.add(&f);
        });
    } else {
        show_empty_view(stack);
    }
    flowbox.show_all();
}

fn setup_podcast_widget(stack: &gtk::Stack) {
    let buidler = gtk::Builder::new_from_string(include_str!("../../gtk/podcast_widget.ui"));
    let pd_widget: gtk::Box = buidler.get_object("podcast_widget").unwrap();

    stack.add_named(&pd_widget, "pdw");
}

fn setup_podcasts_grid(db: &Database, stack: &gtk::Stack) {
    let builder = gtk::Builder::new_from_string(include_str!("../../gtk/podcasts_view.ui"));
    let grid: gtk::Grid = builder.get_object("grid").unwrap();
    stack.add_named(&grid, "pd_grid");
    stack.set_visible_child(&grid);

    // Adapted copy of the way gnome-music does albumview
    let flowbox: gtk::FlowBox = builder.get_object("flowbox").unwrap();

    // TODO: handle unwraps.
    flowbox.connect_child_activated(clone!(db, stack => move |_, child| {
        // This is such an ugly hack...
        let id = child.get_name().unwrap().parse::<i32>().unwrap();
        let parent = {
            let tempdb = db.lock().unwrap();
            dbqueries::get_podcast_from_id(&tempdb, id).unwrap()
        };
        on_flowbox_child_activate(&db, &stack, &parent);
    }));
    // Populate the flowbox with the Podcasts.
    populate_flowbox(db, stack, &flowbox);
}

pub fn setup_stack(db: &Database) -> gtk::Stack {
    let stack = gtk::Stack::new();
    setup_podcast_widget(&stack);
    setup_podcasts_grid(db, &stack);
    stack
}

pub fn update_podcasts_view(db: &Database, stack: &gtk::Stack) {
    let builder = gtk::Builder::new_from_string(include_str!("../../gtk/podcasts_view.ui"));
    let grid: gtk::Grid = builder.get_object("grid").unwrap();

    let flowbox: gtk::FlowBox = builder.get_object("flowbox").unwrap();
    // Populate the flowbox with the Podcasts.
    populate_flowbox(db, stack, &flowbox);

    let old = stack.get_child_by_name("pd_grid").unwrap();
    let vis = stack.get_visible_child_name().unwrap();

    stack.remove(&old);
    stack.add_named(&grid, "pd_grid");
    // preserve the visible widget
    stack.set_visible_child_name(&vis);

    // aggresive memory cleanup
    // probably not needed
    old.destroy();
}

use gtk;
use gtk::prelude::*;
use gdk_pixbuf::Pixbuf;

use hammond_data::dbqueries;
use hammond_data::models::Podcast;
use hammond_data::Database;

use widgets::podcast::*;

fn setup_empty_view(stack: &gtk::Stack) {
    let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/empty_view.ui");
    let view: gtk::Box = builder.get_object("empty_view").unwrap();
    stack.add_named(&view, "empty");
}

fn show_empty_view(stack: &gtk::Stack) {
    stack.set_visible_child_name("empty");

    info!("Empty view.");
}

fn populate_flowbox(db: &Database, flowbox: &gtk::FlowBox) {
    let podcasts = {
        let db = db.lock().unwrap();
        dbqueries::get_podcasts(&db)
    };

    if let Ok(pds) = podcasts {
        pds.iter().for_each(|parent| {
            let f = create_flowbox_child(db, parent);
            flowbox.add(&f);
        });
        flowbox.show_all();
    }
}

fn create_flowbox_child(db: &Database, pd: &Podcast) -> gtk::FlowBoxChild {
    let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/podcasts_child.ui");

    // Copy of gnome-music AlbumWidget
    let box_: gtk::Box = builder.get_object("fb_child").unwrap();
    let pd_title: gtk::Label = builder.get_object("pd_title").unwrap();
    let pd_cover: gtk::Image = builder.get_object("pd_cover").unwrap();
    let banner: gtk::Image = builder.get_object("banner").unwrap();
    let banner_title: gtk::Label = builder.get_object("banner_label").unwrap();

    pd_title.set_text(pd.title());

    let cover = get_pixbuf_from_path(pd);
    if let Some(img) = cover {
        pd_cover.set_from_pixbuf(&img);
    };

    configure_banner(db, pd, &banner, &banner_title);

    let fbc = gtk::FlowBoxChild::new();
    // There's probably a better way to store the id somewhere.
    fbc.set_name(&pd.id().to_string());
    fbc.add(&box_);
    fbc
}

fn configure_banner(db: &Database, pd: &Podcast, banner: &gtk::Image, banner_title: &gtk::Label) {
    let bann = Pixbuf::new_from_resource_at_scale("/org/gnome/hammond/banner.png", 256, 256, true);
    if let Ok(b) = bann {
        banner.set_from_pixbuf(&b);

        let new_episodes = {
            let tempdb = db.lock().unwrap();
            dbqueries::get_pd_unplayed_episodes(&tempdb, pd)
        };

        if let Ok(n) = new_episodes {
            if !n.is_empty() {
                banner_title.set_text(&n.len().to_string());
                banner.show();
                banner_title.show();
            }
        }
    }
}

fn on_flowbox_child_activate(db: &Database, stack: &gtk::Stack, parent: &Podcast) {
    let old = stack.get_child_by_name("pdw").unwrap();
    let pdw = podcast_widget(db, stack, parent);

    stack.remove(&old);
    stack.add_named(&pdw, "pdw");
    stack.set_visible_child(&pdw);

    // aggresive memory cleanup
    // probably not needed
    old.destroy();
}

fn setup_podcasts_flowbox(db: &Database, stack: &gtk::Stack) -> gtk::FlowBox {
    let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/podcasts_view.ui");
    let fb_parent: gtk::Box = builder.get_object("fb_parent").unwrap();
    let flowbox: gtk::FlowBox = builder.get_object("flowbox").unwrap();
    init_flowbox(db, stack, &flowbox);

    stack.add_named(&fb_parent, "fb_parent");

    if flowbox.get_children().is_empty() {
        show_empty_view(stack);
    } else {
        stack.set_visible_child(&fb_parent);
    };

    flowbox
}

pub fn setup_stack(db: &Database) -> gtk::Stack {
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    setup_empty_view(&stack);
    setup_podcast_widget(&stack);
    setup_podcasts_flowbox(db, &stack);
    stack
}

pub fn update_podcasts_view(db: &Database, stack: &gtk::Stack) {
    let vis = stack.get_visible_child_name().unwrap();
    let old = stack.get_child_by_name("fb_parent").unwrap();
    stack.remove(&old);

    let flowbox = setup_podcasts_flowbox(db, stack);

    if vis == "empty" && !flowbox.get_children().is_empty() {
        stack.set_visible_child_name("fb_parent");
    } else if vis == "fb_parent" && flowbox.get_children().is_empty() {
        stack.set_visible_child_name("empty");
    } else {
        // preserve the visible widget
        stack.set_visible_child_name(&vis);
    };

    // aggresive memory cleanup
    // probably not needed
    old.destroy();
}

fn init_flowbox(db: &Database, stack: &gtk::Stack, flowbox: &gtk::FlowBox) {
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
    populate_flowbox(db, flowbox);
}

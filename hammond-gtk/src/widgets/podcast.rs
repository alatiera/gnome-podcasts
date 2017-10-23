use gtk::prelude::*;
use gtk;
use gdk_pixbuf::Pixbuf;

use hammond_data::models::Podcast;
use hammond_downloader::downloader;
use hammond_data::index_feed::Database;

use widgets::episode::episodes_listbox;

pub fn podcast_widget(
    db: &Database,
    title: Option<&str>,
    description: Option<&str>,
    image: Option<Pixbuf>,
) -> gtk::Box {
    // Adapted from gnome-music AlbumWidget
    let pd_widget_source = include_str!("../../gtk/podcast_widget.ui");
    let pd_widget_buidler = gtk::Builder::new_from_string(pd_widget_source);
    let pd_widget: gtk::Box = pd_widget_buidler.get_object("podcast_widget").unwrap();

    let cover: gtk::Image = pd_widget_buidler.get_object("cover").unwrap();
    let title_label: gtk::Label = pd_widget_buidler.get_object("title_label").unwrap();
    let desc_label: gtk::Label = pd_widget_buidler.get_object("description_label").unwrap();
    let view: gtk::Viewport = pd_widget_buidler.get_object("view").unwrap();

    if let Some(t) = title {
        title_label.set_text(t);
        let listbox = episodes_listbox(db, t);
        view.add(&listbox);
    }

    if let Some(d) = description {
        desc_label.set_text(d);
    }

    if let Some(i) = image {
        cover.set_from_pixbuf(&i);
    }

    pd_widget
}

pub fn create_flowbox_child(title: &str, cover: Option<Pixbuf>) -> gtk::FlowBoxChild {
    let build_src = include_str!("../../gtk/podcasts_child.ui");
    let builder = gtk::Builder::new_from_string(build_src);

    // Copy of gnome-music AlbumWidget
    let box_: gtk::Box = builder.get_object("fb_child").unwrap();
    let pd_title: gtk::Label = builder.get_object("pd_title").unwrap();
    let pd_cover: gtk::Image = builder.get_object("pd_cover").unwrap();

    let events: gtk::EventBox = builder.get_object("events").unwrap();

    // GDK.TOUCH_MASK
    // https://developer.gnome.org/gdk3/stable/gdk3-Events.html#GDK-TOUCH-MASK:CAPS
    // http://gtk-rs.org/docs/gdk/constant.TOUCH_MASK.html
    events.add_events(4_194_304);

    pd_title.set_text(title);

    if let Some(img) = cover {
        pd_cover.set_from_pixbuf(&img);
    };

    let fbc = gtk::FlowBoxChild::new();
    fbc.add(&box_);
    // info!("flowbox child created");
    fbc
}

pub fn on_flowbox_child_activate(
    db: &Database,
    stack: &gtk::Stack,
    parent: &Podcast,
    pixbuf: Option<Pixbuf>,
) {
    let old = stack.get_child_by_name("pdw").unwrap();
    let pdw = podcast_widget(db, Some(parent.title()), Some(parent.description()), pixbuf);

    stack.remove(&old);
    stack.add_named(&pdw, "pdw");
    stack.set_visible_child(&pdw);

    // aggresive memory cleanup
    // probably not needed
    old.destroy();
    println!("Hello World!, child activated");
}

pub fn get_pixbuf_from_path(img_path: Option<&str>, pd_title: &str) -> Option<Pixbuf> {
    let img_path = downloader::cache_image(pd_title, img_path);
    if let Some(i) = img_path {
        Pixbuf::new_from_file_at_scale(&i, 200, 200, true).ok()
    } else {
        None
    }
}

// pub fn update_podcast_widget(db: &&Database, stack: &gtk::Stack, pd:
// &Podcast){
//     let old = stack.get_child_by_name("pdw").unwrap();
//     let pdw = pd_widget_from_diesel_model(&db.clone(), pd, &stack.clone());
//         let vis = stack.get_visible_child_name().unwrap();

//     stack.remove(&old);
//     stack.add_named(&pdw, "pdw");
//     stack.set_visible_child_name(&vis);
// }

// pub fn pd_widget_from_diesel_model(db: &Database, pd: &Podcast) -> gtk::Box {
//     let img = get_pixbuf_from_path(pd.image_uri(), pd.title());
//     podcast_widget(db, Some(pd.title()), Some(pd.description()), img)
// }

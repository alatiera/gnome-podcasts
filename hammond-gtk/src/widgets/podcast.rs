use gtk::prelude::*;
use gtk;
use gdk_pixbuf::Pixbuf;

use diesel::prelude::SqliteConnection;
use hammond_data::dbqueries;
use hammond_data::models::Podcast;
use hammond_downloader::downloader;

use std::sync::{Arc, Mutex};

use widgets::episode::episodes_listbox;

pub fn podcast_widget(
    connection: Arc<Mutex<SqliteConnection>>,
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
        let listbox = episodes_listbox(connection, t);
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

// Figure if its better to completly ditch stores and just create the views from diesel models.
pub fn podcast_liststore(connection: &SqliteConnection) -> gtk::ListStore {
    let builder = include_str!("../../gtk/podcast_widget.ui");
    let builder = gtk::Builder::new_from_string(builder);
    let podcast_model: gtk::ListStore = builder.get_object("pd_store").unwrap();

    // TODO: handle unwrap.
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

// pub fn update_podcast_widget(db: &Arc<Mutex<SqliteConnection>>, stack: &gtk::Stack, pd:
// &Podcast){
//     let old = stack.get_child_by_name("pdw").unwrap();
//     let pdw = pd_widget_from_diesel_model(&db.clone(), pd, &stack.clone());

//     stack.remove(&old);
//     stack.add_named(&pdw, "pdw");
//     stack.set_visible_child_full("pdw", StackTransitionType::None);
// }

pub fn pd_widget_from_diesel_model(db: Arc<Mutex<SqliteConnection>>, pd: &Podcast) -> gtk::Box {
    let img = get_pixbuf_from_path(pd.image_uri(), pd.title());
    podcast_widget(db, Some(pd.title()), Some(pd.description()), img)
}

pub fn get_pixbuf_from_path(img_path: Option<&str>, pd_title: &str) -> Option<Pixbuf> {
    let img_path = downloader::cache_image(pd_title, img_path);
    if let Some(i) = img_path {
        Pixbuf::new_from_file_at_scale(&i, 200, 200, true).ok()
    } else {
        None
    }
}

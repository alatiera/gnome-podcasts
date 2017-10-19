use gtk::prelude::*;
use gtk;
use gdk_pixbuf::Pixbuf;

use diesel::prelude::SqliteConnection;
use hammond_data::dbqueries;

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
        let listbox = episodes_listbox(connection.clone(), t);
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
    events.add_events(4194304);

    pd_title.set_text(&title);

    if let Some(img) = cover {
        pd_cover.set_from_pixbuf(&img);
    };

    let fbc = gtk::FlowBoxChild::new();
    fbc.add(&box_);
    // info!("flowbox child created");
    fbc
}

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

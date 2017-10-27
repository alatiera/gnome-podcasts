use gtk::prelude::*;
use gtk;
use gdk_pixbuf::Pixbuf;

use std::fs;

use hammond_data::models::Podcast;
use hammond_downloader::downloader;
use hammond_data::index_feed::Database;
use hammond_data::dbqueries::{load_podcast_from_title, remove_feed};

use widgets::episode::episodes_listbox;
use podcasts_view::update_podcasts_view;

// http://gtk-rs.org/tuto/closures
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

pub fn podcast_widget(
    db: &Database,
    stack: &gtk::Stack,
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
    let unsub_button: gtk::Button = pd_widget_buidler.get_object("unsub_button").unwrap();

    // TODO: refactor, splitoff, spawn a thread probably.
    if title.is_some() {
        let t = title.unwrap().to_owned();
        unsub_button.connect_clicked(clone!(db, stack, t => move |bttn| {
        let pd = {
            let tempdb = db.lock().unwrap();
            load_podcast_from_title(&tempdb, &t)};
        if let Ok(pd) = pd {

        let res = remove_feed(&db, &pd);
        if res.is_ok(){
            info!("{} was removed succesfully.", &t);
            // hack to get away without properly checking for none.
            // if pressed twice would panic.
            bttn.hide();

            let dl_fold = downloader::get_download_folder(&t);
            if let Ok(fold) = dl_fold{
                let res3 = fs::remove_dir_all(&fold);
                if res3.is_ok(){
                    info!("All the content at, {} was removed succesfully", &fold);

                }
            };
        }
        }
        update_podcasts_view(&db, &stack);
        stack.set_visible_child_name("pd_grid")
    }));
    }

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
    let pdw = podcast_widget(
        db,
        stack,
        Some(parent.title()),
        Some(parent.description()),
        pixbuf,
    );

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

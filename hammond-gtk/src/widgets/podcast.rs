use gtk::prelude::*;
use gtk;
use gdk_pixbuf::Pixbuf;

use std::fs;

use hammond_data::models::Podcast;
use hammond_downloader::downloader;
use hammond_data::index_feed::Database;
use hammond_data::dbqueries;

use widgets::episode::episodes_listbox;
use podcasts_view::update_podcasts_view;

fn podcast_widget(db: &Database, stack: &gtk::Stack, pd: &Podcast) -> gtk::Box {
    // Adapted from gnome-music AlbumWidget
    let builder = gtk::Builder::new_from_string(include_str!("../../gtk/podcast_widget.ui"));
    let pd_widget: gtk::Box = builder.get_object("podcast_widget").unwrap();

    let cover: gtk::Image = builder.get_object("cover").unwrap();
    let title_label: gtk::Label = builder.get_object("title_label").unwrap();
    let desc_text_view: gtk::TextView = builder.get_object("desc_text_view").unwrap();
    let view: gtk::Viewport = builder.get_object("view").unwrap();
    let unsub_button: gtk::Button = builder.get_object("unsub_button").unwrap();
    let played_button: gtk::Button = builder.get_object("mark_all_played_button").unwrap();

    // TODO: should spawn a thread to avoid locking the UI probably.
    unsub_button.connect_clicked(clone!(db, stack, pd => move |bttn| {
        on_unsub_button_clicked(&db, &stack, &pd, bttn);
    }));

    title_label.set_text(pd.title());
    let listbox = episodes_listbox(db, pd);
    if let Ok(l) = listbox {
        view.add(&l);
    }

    {
        let buff = desc_text_view.get_buffer().unwrap();
        buff.set_text(pd.description());
    }

    let img = get_pixbuf_from_path(pd.image_uri(), pd.title());
    if let Some(i) = img {
        cover.set_from_pixbuf(&i);
    }

    played_button.connect_clicked(clone!(db, stack, pd => move |_| {
        on_played_button_clicked(&db, &stack, &pd);
    }));

    show_played_button(db, pd, &played_button);

    pd_widget
}

fn on_unsub_button_clicked(
    db: &Database,
    stack: &gtk::Stack,
    pd: &Podcast,
    unsub_button: &gtk::Button,
) {
    let res = dbqueries::remove_feed(db, pd);
    if res.is_ok() {
        info!("{} was removed succesfully.", pd.title());
        // hack to get away without properly checking for none.
        // if pressed twice would panic.
        unsub_button.hide();

        let dl_fold = downloader::get_download_folder(pd.title());
        if let Ok(fold) = dl_fold {
            let res3 = fs::remove_dir_all(&fold);
            if res3.is_ok() {
                info!("All the content at, {} was removed succesfully", &fold);
            }
        };
    }
    update_podcasts_view(db, stack);
    stack.set_visible_child_name("pd_grid")
}

fn on_played_button_clicked(db: &Database, stack: &gtk::Stack, pd: &Podcast) {
    {
        let tempdb = db.lock().unwrap();
        let _ = dbqueries::update_none_to_played_now(&tempdb, pd);
    }

    update_podcast_widget(db, stack, pd);
}

fn show_played_button(db: &Database, pd: &Podcast, played_button: &gtk::Button) {
    let new_episodes = {
        let tempdb = db.lock().unwrap();
        dbqueries::get_pd_unplayed_episodes(&tempdb, pd)
    };

    if let Ok(n) = new_episodes {
        if !n.is_empty() {
            played_button.show()
        }
    }
}

pub fn create_flowbox_child(db: &Database, pd: &Podcast) -> gtk::FlowBoxChild {
    let builder = gtk::Builder::new_from_string(include_str!("../../gtk/podcasts_child.ui"));

    // Copy of gnome-music AlbumWidget
    let box_: gtk::Box = builder.get_object("fb_child").unwrap();
    let pd_title: gtk::Label = builder.get_object("pd_title").unwrap();
    let pd_cover: gtk::Image = builder.get_object("pd_cover").unwrap();
    let banner: gtk::Image = builder.get_object("banner").unwrap();
    let banner_title: gtk::Label = builder.get_object("banner_label").unwrap();

    pd_title.set_text(pd.title());

    let cover = get_pixbuf_from_path(pd.image_uri(), pd.title());
    if let Some(img) = cover {
        pd_cover.set_from_pixbuf(&img);
    };

    configure_banner(db, pd, &banner, &banner_title);

    let fbc = gtk::FlowBoxChild::new();
    fbc.add(&box_);
    // info!("flowbox child created");
    fbc
}

fn configure_banner(db: &Database, pd: &Podcast, banner: &gtk::Image, banner_title: &gtk::Label) {
    // TODO: use GResource instead.
    let bann = Pixbuf::new_from_file_at_scale("hammond-gtk/gtk/banner.png", 256, 256, true);
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

pub fn on_flowbox_child_activate(db: &Database, stack: &gtk::Stack, parent: &Podcast) {
    let old = stack.get_child_by_name("pdw").unwrap();
    let pdw = podcast_widget(db, stack, parent);

    stack.remove(&old);
    stack.add_named(&pdw, "pdw");
    stack.set_visible_child(&pdw);

    // aggresive memory cleanup
    // probably not needed
    old.destroy();
    println!("Hello World!, child activated");
}

fn get_pixbuf_from_path(img_path: Option<&str>, pd_title: &str) -> Option<Pixbuf> {
    let img_path = downloader::cache_image(pd_title, img_path);
    if let Some(i) = img_path {
        Pixbuf::new_from_file_at_scale(&i, 256, 256, true).ok()
    } else {
        None
    }
}

pub fn update_podcast_widget(db: &Database, stack: &gtk::Stack, pd: &Podcast) {
    let old = stack.get_child_by_name("pdw").unwrap();
    let pdw = podcast_widget(db, stack, pd);
    let vis = stack.get_visible_child_name().unwrap();

    stack.remove(&old);
    stack.add_named(&pdw, "pdw");
    stack.set_visible_child_name(&vis);
}


#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]

use open;
use diesel::prelude::SqliteConnection;
use hammond_data::dbqueries;
use hammond_data::models::Episode;
use hammond_downloader::downloader;
// use html5ever::parse_document;
// use html5ever::rcdom::RcDom;
// use tendril::stream::TendrilSink;
use dissolve::strip_html_tags;

use std::thread;
use std::sync::{Arc, Mutex};

use gtk;
use gtk::prelude::*;

// use utils;

fn epidose_widget(
    connection: &Arc<Mutex<SqliteConnection>>,
    episode: &mut Episode,
    pd_title: &str,
) -> gtk::Box {
    // This is just a prototype and will be reworked probably.
    let builder = include_str!("../../gtk/episode_widget.ui");
    let builder = gtk::Builder::new_from_string(builder);

    let ep: gtk::Box = builder.get_object("episode_box").unwrap();
    let dl_button: gtk::Button = builder.get_object("download_button").unwrap();
    let play_button: gtk::Button = builder.get_object("play_button").unwrap();

    let title_label: gtk::Label = builder.get_object("title_label").unwrap();
    let desc_label: gtk::Label = builder.get_object("desc_label").unwrap();
    let expander: gtk::Expander = builder.get_object("expand_desc").unwrap();

    title_label.set_xalign(0.0);
    desc_label.set_xalign(0.0);

    if let Some(t) = episode.title() {
        title_label.set_text(t);
    }

    if episode.description().is_some() {
        // let mk = utils::html_to_markup(d.to_string());
        // desc_label.set_markup(mk.as_str());
        let d = episode.description().unwrap().to_owned();

        expander.connect_activate(move |_| {
            let plain_text = strip_html_tags(&d).join(" ");
            desc_label.set_text(plain_text.trim())
        });
    }

    if episode.local_uri().is_some() {
        dl_button.hide();
        play_button.show();
        let uri = episode.local_uri().unwrap().to_owned();
        play_button.connect_clicked(move |_| {
            let e = open::that(&uri);
            if e.is_err() {
                error!("Error while trying to open: {}", uri);
            }
        });
    }

    let pd_title_cloned = pd_title.to_owned();
    let db = connection.clone();
    let ep_clone = episode.clone();
    dl_button.connect_clicked(move |_| {
        // ugly hack to bypass the borrowchecker
        let pd_title = pd_title_cloned.clone();
        let db = db.clone();
        let mut ep_clone = ep_clone.clone();

        thread::spawn(move || {
            let dl_fold = downloader::get_dl_folder(&pd_title).unwrap();
            let tempdb = db.lock().unwrap();
            let e = downloader::get_episode(&tempdb, &mut ep_clone, dl_fold.as_str());
            if let Err(err) = e {
                error!("Error while trying to download: {}", ep_clone.uri());
                error!("Error: {}", err);
            };
            // TODO: emit a signal in order to update the podcast widget.
        });
    });

    ep
}

pub fn episodes_listbox(connection: &Arc<Mutex<SqliteConnection>>, pd_title: &str) -> gtk::ListBox {
    // TODO: handle unwraps.
    let m = connection.lock().unwrap();
    let pd = dbqueries::load_podcast(&m, pd_title).unwrap();
    let mut episodes = dbqueries::get_pd_episodes(&m, &pd).unwrap();
    drop(m);

    let list = gtk::ListBox::new();
    episodes.iter_mut().for_each(|ep| {
        let w = epidose_widget(&connection.clone(), ep, pd_title);
        list.add(&w)
    });

    list.set_vexpand(false);
    list.set_hexpand(false);
    list.set_visible(true);
    list
}

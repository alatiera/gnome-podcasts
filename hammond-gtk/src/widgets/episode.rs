
#![cfg_attr(feature = "cargo-clippy", allow(clone_on_ref_ptr))]
#![cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]

use open;
use diesel::prelude::SqliteConnection;
use hammond_data::dbqueries;
use hammond_data::models::Episode;
use hammond_downloader::downloader;
use dissolve::strip_html_tags;

use std::thread;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver};
use std::path::Path;

use glib;
use gtk;
use gtk::prelude::*;
use gtk::ContainerExt;

thread_local!(
    static GLOBAL: RefCell<Option<((gtk::Button,
    gtk::Button,
    Receiver<bool>))>> = RefCell::new(None));

// TODO: REFACTOR AND MODULATE ME.
fn epidose_widget(
    connection: Arc<Mutex<SqliteConnection>>,
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
        let d = episode.description().unwrap().to_owned();

        expander.connect_activate(move |_| {
            let plain_text = strip_html_tags(&d).join(" ");
            desc_label.set_text(plain_text.trim())
        });
    }

    if episode.local_uri().is_some() {
        let uri = episode.local_uri().unwrap().to_owned();

        if Path::new(&uri).exists() {
            dl_button.hide();
            play_button.show();
            play_button.connect_clicked(move |_| {
                let e = open::that(&uri);
                if e.is_err() {
                    error!("Error while trying to open: {}", uri);
                }
            });
        }
    }

    // TODO: figure out how to use the gtk-clone macro,
    // to make it less tedious.
    let pd_title_clone = pd_title.to_owned();
    let db = connection.clone();
    let ep_clone = episode.clone();
    let play_button_clone = play_button.clone();
    let dl_button_clone = dl_button.clone();
    dl_button.connect_clicked(move |_| {
        on_dl_clicked(
            db.clone(),
            &pd_title_clone,
            &mut ep_clone.clone(),
            dl_button_clone.clone(),
            play_button_clone.clone(),
        );
    });

    ep
}

// TODO: show notification when dl is finished and block play_bttn till then.
fn on_dl_clicked(
    db: Arc<Mutex<SqliteConnection>>,
    pd_title: &str,
    ep: &mut Episode,
    dl_bttn: gtk::Button,
    play_bttn: gtk::Button,
) {
    // Create a async channel.
    let (sender, receiver) = channel();

    // Pass the desired arguments into the Local Thread Storage.
    GLOBAL.with(move |global| {
        *global.borrow_mut() = Some((dl_bttn, play_bttn, receiver));
    });

    let pd_title = pd_title.to_owned();
    let mut ep = ep.clone();
    thread::spawn(move || {
        let dl_fold = downloader::get_dl_folder(&pd_title).unwrap();
        let e = downloader::get_episode(db, &mut ep, dl_fold.as_str());
        if let Err(err) = e {
            error!("Error while trying to download: {}", ep.uri());
            error!("Error: {}", err);
        };
        sender.send(true).expect("Couldn't send data to channel");;
        glib::idle_add(receive);
    });
}

fn receive() -> glib::Continue {
    GLOBAL.with(|global| {
        if let Some((ref dl_bttn, ref play_bttn, ref reciever)) = *global.borrow() {
            if reciever.try_recv().is_ok() {
                dl_bttn.hide();
                play_bttn.show();
            }
        }
    });
    glib::Continue(false)
}

pub fn episodes_listbox(connection: Arc<Mutex<SqliteConnection>>, pd_title: &str) -> gtk::ListBox {
    // TODO: handle unwraps.
    let m = connection.lock().unwrap();
    let pd = dbqueries::load_podcast(&m, pd_title).unwrap();
    let mut episodes = dbqueries::get_pd_episodes(&m, &pd).unwrap();
    drop(m);

    let list = gtk::ListBox::new();
    episodes.iter_mut().for_each(|ep| {
        let w = epidose_widget(connection.clone(), ep, pd_title);
        list.add(&w)
    });

    list.set_vexpand(false);
    list.set_hexpand(false);
    list.set_visible(true);
    list
}

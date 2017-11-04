
use open;
use hammond_data::dbqueries;
use hammond_data::models::{Episode, Podcast};
use hammond_downloader::downloader;
use hammond_data::index_feed::Database;
use hammond_data::dbcheckup::*;
use hammond_data::errors::*;

use dissolve::strip_html_tags;

use std::thread;
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver};
use std::path::Path;

use glib;
use gtk;
use gtk::prelude::*;
use gtk::{ContainerExt, TextBufferExt};

thread_local!(
    static GLOBAL: RefCell<Option<((
        gtk::Button, gtk::Button, gtk::Button, Receiver<bool>,
        ))>> = RefCell::new(None));

fn epidose_widget(db: &Database, episode: &mut Episode, pd_title: &str) -> gtk::Box {
    // This is just a prototype and will be reworked probably.
    let builder = gtk::Builder::new_from_string(include_str!("../../gtk/episode_widget.ui"));

    let ep: gtk::Box = builder.get_object("episode_box").unwrap();
    let download_button: gtk::Button = builder.get_object("download_button").unwrap();
    let play_button: gtk::Button = builder.get_object("play_button").unwrap();
    let delete_button: gtk::Button = builder.get_object("delete_button").unwrap();
    let played_button: gtk::Button = builder.get_object("mark_played_button").unwrap();
    let unplayed_button: gtk::Button = builder.get_object("mark_unplayed_button").unwrap();

    let title_label: gtk::Label = builder.get_object("title_label").unwrap();
    // let desc_label: gtk::Label = builder.get_object("desc_label").unwrap();
    let expander: gtk::Expander = builder.get_object("expand_desc").unwrap();
    let desc_text_view: gtk::TextView = builder.get_object("desc_text_view").unwrap();

    title_label.set_xalign(0.0);

    if let Some(t) = episode.title() {
        title_label.set_text(t);
    }

    if episode.description().is_some() {
        let d = episode.description().unwrap().to_owned();

        expander.connect_activate(move |_| {
            let plain_text = strip_html_tags(&d).join(" ");
            // TODO: handle unwrap
            let buff = desc_text_view.get_buffer().unwrap();
            buff.set_text(plain_text.trim());
        });
    }

    if episode.played().is_some() {
        unplayed_button.show();
        played_button.hide();
    }

    // Show or hide the play/delete/download buttons upon widget initialization.
    let local_uri = episode.local_uri();
    if local_uri.is_some() && Path::new(local_uri.unwrap()).exists() {
        download_button.hide();
        play_button.show();
        delete_button.show();
    }

    play_button.connect_clicked(clone!(db, episode => move |_| {
        on_play_bttn_clicked(&db, episode.id());
        let _ = set_played_now(&db, &mut episode.clone());
    }));

    delete_button.connect_clicked(
        clone!(episode, db, play_button, download_button => move |del| {
        on_delete_bttn_clicked(&db, episode.id());
        del.hide();
        play_button.hide();
        download_button.show();
    }),
    );

    played_button.connect_clicked(clone!(db, episode, unplayed_button => move |played| {
        let _ = set_played_now(&db, &mut episode.clone());
        played.hide();
        unplayed_button.show();
    }));

    unplayed_button.connect_clicked(clone!(db, episode, played_button => move |un| {
        let mut episode = episode.clone();
        episode.set_played(None);
        let _ = episode.save(&db);
        un.hide();
        played_button.show();
    }));

    let pd_title = pd_title.to_owned();
    download_button.connect_clicked(
        clone!(db, play_button, delete_button, episode  => move |dl| {
        on_download_clicked(
            &db,
            &pd_title,
            &mut episode.clone(),
            dl,
            &play_button,
            &delete_button,
        );
    }),
    );

    ep
}

// TODO: show notification when dl is finished.
fn on_download_clicked(
    db: &Database,
    pd_title: &str,
    ep: &mut Episode,
    download_bttn: &gtk::Button,
    play_bttn: &gtk::Button,
    del_bttn: &gtk::Button,
) {
    // Create a async channel.
    let (sender, receiver) = channel();

    // Pass the desired arguments into the Local Thread Storage.
    GLOBAL.with(clone!(download_bttn, play_bttn, del_bttn => move |global| {
        *global.borrow_mut() = Some((download_bttn, play_bttn, del_bttn, receiver));
    }));

    let pd_title = pd_title.to_owned();
    let mut ep = ep.clone();
    thread::spawn(clone!(db => move || {
        let download_fold = downloader::get_download_folder(&pd_title).unwrap();
        let e = downloader::get_episode(&db, &mut ep, download_fold.as_str());
        if let Err(err) = e {
            error!("Error while trying to download: {}", ep.uri());
            error!("Error: {}", err);
        };
        sender.send(true).expect("Couldn't send data to channel");;
        glib::idle_add(receive);
    }));
}

fn on_play_bttn_clicked(db: &Database, episode_id: i32) {
    let local_uri = {
        let tempdb = db.lock().unwrap();
        dbqueries::get_episode_local_uri(&tempdb, episode_id).unwrap()
    };

    if let Some(uri) = local_uri {
        if Path::new(&uri).exists() {
            info!("Opening {}", uri);
            let e = open::that(&uri);
            if let Err(err) = e {
                error!("Error while trying to open file: {}", uri);
                error!("Error: {}", err);
            };
        }
    } else {
        error!(
            "Something went wrong evaluating the following path: {:?}",
            local_uri
        );
    }
}

fn on_delete_bttn_clicked(db: &Database, episode_id: i32) {
    let mut ep = {
        let tempdb = db.lock().unwrap();
        dbqueries::get_episode(&tempdb, episode_id).unwrap()
    };

    let e = delete_local_content(db, &mut ep);
    if let Err(err) = e {
        error!("Error while trying to delete file: {:?}", ep.local_uri());
        error!("Error: {}", err);
    };
}

fn receive() -> glib::Continue {
    GLOBAL.with(|global| {
        if let Some((ref download_bttn, ref play_bttn, ref del_bttn, ref reciever)) =
            *global.borrow()
        {
            if reciever.try_recv().is_ok() {
                download_bttn.hide();
                play_bttn.show();
                del_bttn.show();
            }
        }
    });
    glib::Continue(false)
}

pub fn episodes_listbox(db: &Database, pd: &Podcast) -> Result<gtk::ListBox> {
    let conn = db.lock().unwrap();
    let mut episodes = dbqueries::get_pd_episodes(&conn, &pd)?;
    drop(conn);

    let list = gtk::ListBox::new();
    episodes.iter_mut().for_each(|ep| {
        let w = epidose_widget(db, ep, pd.title());
        list.add(&w)
    });

    list.set_vexpand(false);
    list.set_hexpand(false);
    list.set_visible(true);
    list.set_selection_mode(gtk::SelectionMode::None);
    Ok(list)
}

use glib;
use gtk;
use gtk::prelude::*;
use gtk::{ContainerExt, TextBufferExt};

use open;
use dissolve::strip_html_tags;
use diesel::associations::Identifiable;

use hammond_data::dbqueries;
use hammond_data::{Episode, Podcast};
use hammond_downloader::downloader;
use hammond_data::utils::*;
use hammond_data::errors::*;
use hammond_data::utils::replace_extra_spaces;

// use utils::html_to_markup;

use std::thread;
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver};
use std::path::Path;

type Foo = RefCell<Option<(gtk::Button, gtk::Button, gtk::Button, Receiver<bool>)>>;

thread_local!(static GLOBAL: Foo = RefCell::new(None));

#[derive(Debug)]
struct EpisodeWidget {
    container: gtk::Box,
    download: gtk::Button,
    play: gtk::Button,
    delete: gtk::Button,
    played: gtk::Button,
    unplayed: gtk::Button,
    title: gtk::Label,
    description: gtk::TextView,
    // description: gtk::Label,
    expander: gtk::Expander,
}

impl EpisodeWidget {
    fn new() -> EpisodeWidget {
        // This is just a prototype and will be reworked probably.
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/episode_widget.ui");

        let container: gtk::Box = builder.get_object("episode_box").unwrap();
        let download: gtk::Button = builder.get_object("download_button").unwrap();
        let play: gtk::Button = builder.get_object("play_button").unwrap();
        let delete: gtk::Button = builder.get_object("delete_button").unwrap();
        let played: gtk::Button = builder.get_object("mark_played_button").unwrap();
        let unplayed: gtk::Button = builder.get_object("mark_unplayed_button").unwrap();

        let title: gtk::Label = builder.get_object("title_label").unwrap();
        let expander: gtk::Expander = builder.get_object("expand_desc").unwrap();
        let description: gtk::TextView = builder.get_object("desc_text_view").unwrap();
        // let description: gtk::Label = builder.get_object("desc_text").unwrap();

        EpisodeWidget {
            container,
            download,
            play,
            delete,
            played,
            unplayed,
            title,
            expander,
            description,
        }
    }

    pub fn new_initialized(episode: &mut Episode, pd: &Podcast) -> EpisodeWidget {
        let widget = EpisodeWidget::new();
        widget.init(episode, pd);
        widget
    }

    fn init(&self, episode: &mut Episode, pd: &Podcast) {
        self.title.set_xalign(0.0);

        if let Some(t) = episode.title() {
            self.title.set_text(t);
        }

        if episode.description().is_some() {
            let text = episode.description().unwrap().to_owned();
            let description = &self.description;
            self.expander
                .connect_activate(clone!(description, text => move |_| {
                // let mut text = text.clone();
                // html_to_markup(&mut text);
                // description.set_markup(&text)

                let plain_text = strip_html_tags(&text).join(" ");
                // TODO: handle unwrap
                let buff = description.get_buffer().unwrap();
                buff.set_text(&replace_extra_spaces(&plain_text));
            }));
        }

        if episode.played().is_some() {
            self.unplayed.show();
            self.played.hide();
        }

        // Show or hide the play/delete/download buttons upon widget initialization.
        let local_uri = episode.local_uri();
        if local_uri.is_some() && Path::new(local_uri.unwrap()).exists() {
            self.download.hide();
            self.play.show();
            self.delete.show();
        }

        let played = &self.played;
        let unplayed = &self.unplayed;
        self.play
            .connect_clicked(clone!(episode, played, unplayed => move |_| {
            let mut episode = episode.clone();
            on_play_bttn_clicked(*episode.id());
            let _ = episode.set_played_now();
            played.hide();
            unplayed.show();
        }));

        let play = &self.play;
        let download = &self.download;
        self.delete
            .connect_clicked(clone!(episode, play, download => move |del| {
            on_delete_bttn_clicked(*episode.id());
            del.hide();
            play.hide();
            download.show();
        }));

        let unplayed = &self.unplayed;
        self.played
            .connect_clicked(clone!(episode, unplayed => move |played| {
            let mut episode = episode.clone();
            let _ = episode.set_played_now();
            played.hide();
            unplayed.show();
        }));

        let played = &self.played;
        self.unplayed
            .connect_clicked(clone!(episode, played => move |un| {
            let mut episode = episode.clone();
            episode.set_played(None);
            let _ = episode.save();
            un.hide();
            played.show();
        }));

        let pd_title = pd.title().to_owned();
        let play = &self.play;
        let delete = &self.delete;
        self.download
            .connect_clicked(clone!(play, delete, episode  => move |dl| {
            on_download_clicked(
                &pd_title,
                &mut episode.clone(),
                dl,
                &play,
                &delete,
            );
        }));
    }
}

// TODO: show notification when dl is finished.
fn on_download_clicked(
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
    thread::spawn(move || {
        let download_fold = downloader::get_download_folder(&pd_title).unwrap();
        let e = downloader::get_episode(&mut ep, download_fold.as_str());
        if let Err(err) = e {
            error!("Error while trying to download: {}", ep.uri());
            error!("Error: {}", err);
        };
        sender.send(true).expect("Couldn't send data to channel");;
        glib::idle_add(receive);
    });
}

fn on_play_bttn_clicked(episode_id: i32) {
    let local_uri = dbqueries::get_episode_local_uri_from_id(episode_id).unwrap();

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

fn on_delete_bttn_clicked(episode_id: i32) {
    let mut ep = dbqueries::get_episode_from_id(episode_id).unwrap();

    let e = delete_local_content(&mut ep);
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

pub fn episodes_listbox(pd: &Podcast) -> Result<gtk::ListBox> {
    let episodes = dbqueries::get_pd_episodes(pd)?;

    let list = gtk::ListBox::new();
    episodes.into_iter().for_each(|mut ep| {
        // let w = epidose_widget(&mut ep, pd.title());
        let widget = EpisodeWidget::new_initialized(&mut ep, pd);
        list.add(&widget.container)
    });

    list.set_vexpand(false);
    list.set_hexpand(false);
    list.set_visible(true);
    list.set_selection_mode(gtk::SelectionMode::None);
    Ok(list)
}

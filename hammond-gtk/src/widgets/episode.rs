use glib;
use gtk;

use gtk::prelude::*;
use chrono::prelude::*;

use open;
use humansize::{file_size_opts as size_opts, FileSize};

use hammond_data::dbqueries;
use hammond_data::{EpisodeWidgetQuery, Podcast};
// use hammond_data::utils::*;
use hammond_data::errors::*;
use hammond_downloader::downloader;

use app::DOWNLOADS_MANAGER;

use std::path::Path;

#[derive(Debug, Clone)]
pub struct EpisodeWidget {
    pub container: gtk::Box,
    play: gtk::Button,
    download: gtk::Button,
    cancel: gtk::Button,
    title: gtk::Label,
    date: gtk::Label,
    duration: gtk::Label,
    size: gtk::Label,
    progress: gtk::ProgressBar,
    progress_label: gtk::Label,
    separator1: gtk::Label,
    separator2: gtk::Label,
}

impl Default for EpisodeWidget {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/episode_widget.ui");

        let container: gtk::Box = builder.get_object("episode_container").unwrap();
        let progress: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();

        let download: gtk::Button = builder.get_object("download_button").unwrap();
        let play: gtk::Button = builder.get_object("play_button").unwrap();
        let cancel: gtk::Button = builder.get_object("cancel_button").unwrap();

        let title: gtk::Label = builder.get_object("title_label").unwrap();
        let date: gtk::Label = builder.get_object("date_label").unwrap();
        let duration: gtk::Label = builder.get_object("duration_label").unwrap();
        let size: gtk::Label = builder.get_object("size_label").unwrap();
        let progress_label: gtk::Label = builder.get_object("progress_label").unwrap();

        let separator1: gtk::Label = builder.get_object("separator1").unwrap();
        let separator2: gtk::Label = builder.get_object("separator2").unwrap();

        EpisodeWidget {
            container,
            progress,
            download,
            play,
            cancel,
            title,
            duration,
            size,
            date,
            progress_label,
            separator1,
            separator2,
        }
    }
}

impl EpisodeWidget {
    pub fn new(episode: &mut EpisodeWidgetQuery) -> EpisodeWidget {
        let widget = EpisodeWidget::default();
        widget.init(episode);
        widget
    }

    // TODO: calculate lenght.
    // TODO: wire the progress_bar to the downloader.
    // TODO: wire the cancel button.
    fn init(&self, episode: &mut EpisodeWidgetQuery) {
        self.title.set_xalign(0.0);
        self.title.set_text(episode.title());

        if episode.played().is_some() {
            self.title
                .get_style_context()
                .map(|c| c.add_class("dim-label"));
        }

        {
            let m = DOWNLOADS_MANAGER.lock().unwrap();
            let list = m.active.lock().unwrap();
            if list.contains(&episode.rowid()) {
                self.show_progess_bar()
            };
        }

        // Declare a custom humansize option struct
        // See: https://docs.rs/humansize/1.0.2/humansize/file_size_opts/struct.FileSizeOpts.html
        let custom_options = size_opts::FileSizeOpts {
            divider: size_opts::Kilo::Binary,
            units: size_opts::Kilo::Decimal,
            decimal_places: 0,
            decimal_zeroes: 0,
            fixed_at: size_opts::FixedAt::No,
            long_units: false,
            space: true,
            suffix: "",
            allow_negative: false,
        };

        if let Some(size) = episode.length() {
            if size != 0 {
                let s = size.file_size(custom_options);
                if let Ok(s) = s {
                    self.size.set_text(&s);
                    self.size.show();
                    self.separator2.show();
                }
            }
        };

        if let Some(secs) = episode.duration() {
            self.duration.set_text(&format!("{} min", secs / 60));
            self.duration.show();
            self.separator1.show();
        };

        let now = Utc::now();
        let date = Utc.timestamp(i64::from(episode.epoch()), 0);
        if now.year() == date.year() {
            self.date.set_text(&date.format("%e %b").to_string());
        } else {
            self.date.set_text(&date.format("%e %b %Y").to_string());
        };

        // Show or hide the play/delete/download buttons upon widget initialization.
        let local_uri = episode.local_uri();
        if local_uri.is_some() && Path::new(local_uri.unwrap()).exists() {
            self.download.hide();
            self.play.show();
        }

        let title = &self.title;
        self.play
            .connect_clicked(clone!(episode, title => move |_| {
            let mut episode = episode.clone();
            on_play_bttn_clicked(episode.rowid());
            if episode.set_played_now().is_ok() {
                title
                    .get_style_context()
                    .map(|c| c.add_class("dim-label"));
            };
        }));

        let cancel = &self.cancel;
        let progress = self.progress.clone();
        self.download
            .connect_clicked(clone!(episode, cancel, progress  => move |dl| {
            on_download_clicked(
                &episode,
                dl,
                &cancel,
                progress.clone()
            );
        }));
    }

    fn show_progess_bar(&self) {
        let progress_bar = self.progress.clone();
        timeout_add(200, move || {
            progress_bar.pulse();
            glib::Continue(true)
        });

        self.progress.show();
        self.download.hide();
        self.cancel.show();
    }
}

fn on_download_clicked(
    ep: &EpisodeWidgetQuery,
    download_bttn: &gtk::Button,
    cancel_bttn: &gtk::Button,
    progress_bar: gtk::ProgressBar,
) {
    let progress = progress_bar.clone();

    // Start the proggress_bar pulse.
    timeout_add(200, move || {
        progress_bar.pulse();
        glib::Continue(true)
    });

    let pd = dbqueries::get_podcast_from_id(ep.podcast_id()).unwrap();
    let pd_title = pd.title().to_owned();
    cancel_bttn.show();
    progress.show();
    download_bttn.hide();
    let download_fold = downloader::get_download_folder(&pd_title).unwrap();
    {
        let man = DOWNLOADS_MANAGER.lock().unwrap();
        man.add(ep.rowid(), &download_fold);
    }
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

// fn on_delete_bttn_clicked(episode_id: i32) {
//     let mut ep = dbqueries::get_episode_from_rowid(episode_id)
//         .unwrap()
//         .into();

//     let e = delete_local_content(&mut ep);
//     if let Err(err) = e {
//         error!("Error while trying to delete file: {:?}", ep.local_uri());
//         error!("Error: {}", err);
//     };
// }

pub fn episodes_listbox(pd: &Podcast) -> Result<gtk::ListBox> {
    let mut episodes = dbqueries::get_pd_episodeswidgets(pd)?;

    let list = gtk::ListBox::new();

    episodes.iter_mut().for_each(|ep| {
        let widget = EpisodeWidget::new(ep);
        list.add(&widget.container);
    });

    list.set_vexpand(false);
    list.set_hexpand(false);
    list.set_visible(true);
    list.set_selection_mode(gtk::SelectionMode::None);
    Ok(list)
}

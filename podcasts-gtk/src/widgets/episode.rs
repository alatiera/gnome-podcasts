// episode.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use adw::subclass::prelude::*;
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;

use anyhow::{anyhow, Result};
use chrono::prelude::*;
use glib::Sender;
use humansize::{file_size_opts as size_opts, FileSize};
use once_cell::sync::Lazy;

use podcasts_data::dbqueries;
use podcasts_data::downloader::DownloadProgress;
use podcasts_data::utils::get_download_dir;
use podcasts_data::EpisodeWidgetModel;

use crate::app::Action;
use crate::manager;

use std::sync::{Arc, Mutex, TryLockError};
use std::time::Duration;

use crate::i18n::i18n_f;

static SIZE_OPTS: Lazy<Arc<size_opts::FileSizeOpts>> = Lazy::new(|| {
    // Declare a custom humansize option struct
    // See: https://docs.rs/humansize/1.0.2/humansize/file_size_opts/struct.FileSizeOpts.html
    Arc::new(size_opts::FileSizeOpts {
        divider: size_opts::Kilo::Binary,
        units: size_opts::Kilo::Decimal,
        decimal_places: 0,
        decimal_zeroes: 0,
        fixed_at: size_opts::FixedAt::No,
        long_units: false,
        space: true,
        suffix: "",
        allow_negative: false,
    })
});

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/episode_widget.ui")]
pub struct EpisodeWidgetPriv {
    #[template_child]
    progressbar: TemplateChild<gtk::ProgressBar>,

    // InfoLabels
    #[template_child]
    title: TemplateChild<gtk::Label>,
    #[template_child]
    date: TemplateChild<gtk::Label>,
    #[template_child]
    separator1: TemplateChild<gtk::Label>,
    #[template_child]
    duration: TemplateChild<gtk::Label>,
    #[template_child]
    separator2: TemplateChild<gtk::Label>,
    #[template_child]
    local_size: TemplateChild<gtk::Label>,
    #[template_child]
    size_separator: TemplateChild<gtk::Label>,
    #[template_child]
    total_size: TemplateChild<gtk::Label>,
    #[template_child]
    played_checkmark: TemplateChild<gtk::Image>,

    // Buttons
    #[template_child]
    play: TemplateChild<gtk::Button>,
    #[template_child]
    download: TemplateChild<gtk::Button>,
    #[template_child]
    cancel: TemplateChild<gtk::Button>,
}

impl EpisodeWidgetPriv {
    pub(crate) fn init(&self, sender: &Sender<Action>, episode: &EpisodeWidgetModel) {
        self.init_info(episode);
        self.determine_buttons_state(episode, sender)
            .map_err(|err| error!("Error: {}", err))
            .ok();
    }

    // InProgress State:
    //   * Show ProgressBar and Cancel Button.
    //   * Show `total_size`, `local_size` labels and `size_separator`.
    //   * Hide Download and Play Buttons
    fn state_prog(&self) {
        self.progressbar.set_visible(true);
        self.cancel.set_visible(true);

        self.total_size.set_visible(true);
        self.local_size.set_visible(true);
        self.size_separator.set_visible(true);

        self.play.set_visible(false);
        self.download.set_visible(false);
    }

    // Playable State:
    //   * Hide ProgressBar and Cancel, Download Buttons.
    //   * Hide `local_size` labels and `size_separator`.
    //   * Show Play Button and `total_size` label
    fn state_playable(&self) {
        self.progressbar.set_visible(false);
        self.cancel.set_visible(false);
        self.download.set_visible(false);
        self.local_size.set_visible(false);
        self.size_separator.set_visible(false);

        self.total_size.set_visible(true);
        self.play.set_visible(true);
    }

    // ToDownload State:
    //   * Hide ProgressBar and Cancel, Play Buttons.
    //   * Hide `local_size` labels and `size_separator`.
    //   * Show Download Button
    //   * Determine `total_size` label state (Comes from `episode.lenght`).
    fn state_download(&self) {
        self.progressbar.set_visible(false);
        self.cancel.set_visible(false);
        self.play.set_visible(false);

        self.local_size.set_visible(false);
        self.size_separator.set_visible(false);

        self.download.set_visible(true);
    }

    fn update_progress(&self, local_size: &str, fraction: f64) {
        self.local_size.set_text(local_size);
        self.progressbar.set_fraction(fraction);
    }

    /// Change the state of the `EpisodeWidget`.
    ///
    /// Function Flowchart:
    ///
    /// -------------------       --------------
    /// | Is the Episode  |  YES  |   State:   |
    /// | currently being | ----> | InProgress |
    /// |   downloaded?   |       |            |
    /// -------------------       --------------
    ///         |
    ///         | NO
    ///         |
    ///        \_/
    /// -------------------       --------------
    /// | is the episode  |  YES  |   State:   |
    /// |   downloaded    | ----> |  Playable  |
    /// |    already?     |       |            |
    /// -------------------       --------------
    ///         |
    ///         | NO
    ///         |
    ///        \_/
    /// -------------------
    /// |     State:      |
    /// |   ToDownload    |
    /// -------------------
    fn determine_buttons_state(
        &self,
        episode: &EpisodeWidgetModel,
        sender: &Sender<Action>,
    ) -> Result<()> {
        // Reset the buttons state no matter the glade file.
        // This is just to make it easier to port to relm in the future.
        self.cancel.set_visible(false);
        self.play.set_visible(false);
        self.download.set_visible(false);

        // Check if the episode is being downloaded
        let id = episode.rowid();
        let active_dl = move || -> Result<Option<_>> {
            let m = manager::ACTIVE_DOWNLOADS
                .read()
                .map_err(|_| anyhow!("Failed to get a lock on the mutex."))?;

            Ok(m.get(&id).cloned())
        };

        // State: InProgress
        if let Some(prog) = active_dl()? {
            // set a callback that will update the state when the download finishes
            let callback = clone!(@weak self as this, @strong sender => @default-return glib::Continue(false), move || {
                if let Ok(guard) = manager::ACTIVE_DOWNLOADS.read() {
                    if !guard.contains_key(&id) {
                        if let Ok(ep) = dbqueries::get_episode_widget_from_rowid(id) {
                            this.determine_buttons_state(&ep, &sender)
                                .map_err(|err| error!("Error: {}", err))
                                .ok();

                            return glib::Continue(false)
                        }
                    }
                }

                glib::Continue(true)
            });
            glib::timeout_add_local(Duration::from_millis(250), callback);

            // Wire the cancel button
            self
                .cancel
                .connect_clicked(clone!(@strong prog, @weak self as this, @strong sender => move |_| {
                    // Cancel the download
                    if let Ok(mut m) = prog.lock() {
                        m.cancel();
                    }

                    // Cancel is not instant so we have to wait a bit
                    glib::timeout_add_local(Duration::from_millis(50), clone!(@weak this, @strong sender => @default-return glib::Continue(false), move || {
                        if let Ok(thing) = active_dl() {
                            if thing.is_none() {
                                // Recalculate the widget state
                                dbqueries::get_episode_widget_from_rowid(id)
                                    .map_err(From::from)
                                    .and_then(|ep| this.determine_buttons_state(&ep, &sender))
                                    .map_err(|err| error!("Error: {}", err))
                                    .ok();

                                return glib::Continue(false)
                            }
                        }

                        glib::Continue(true)
                    }));
                }));

            // Setup a callback that will update the total_size label
            // with the http ContentLength header number rather than
            // relying to the RSS feed.
            update_total_size_callback(self, &prog);

            // Setup a callback that will update the progress bar.
            update_progressbar_callback(self, &prog, id);

            // Change the widget layout/state
            self.state_prog();

            return Ok(());
        }

        // State: Playable
        if episode.local_uri().is_some() {
            // Change the widget layout/state
            self.state_playable();

            // Wire the play button
            self.play
                .connect_clicked(clone!(@weak self as this, @strong sender => move |_| {
                    if let Ok(mut ep) = dbqueries::get_episode_widget_from_rowid(id) {
                        this.on_play_bttn_clicked(&mut ep, &sender)
                            .map_err(|err| error!("Error: {}", err))
                            .ok();
                    }
                }));

            return Ok(());
        }

        // State: ToDownload
        // Wire the download button
        self.download
            .connect_clicked(clone!(@weak self as this, @strong sender => move |dl| {
                if let Ok(ep) = dbqueries::get_episode_widget_from_rowid(id) {
                    on_download_clicked(&ep, &sender)
                        .and_then(|_| {
                            info!("Download started successfully.");
                            this.determine_buttons_state(&ep, &sender)
                        })
                        .map_err(|err| error!("Error: {}", err))
                        .ok();
                }

                // Restore sensitivity after operations above complete
                dl.set_sensitive(true);
            }));

        // Change the widget state into `ToDownload`
        self.state_download();

        Ok(())
    }

    fn on_play_bttn_clicked(
        &self,
        episode: &mut EpisodeWidgetModel,
        sender: &Sender<Action>,
    ) -> Result<()> {
        // Grey out the title
        self.set_title(episode);

        // Play the episode
        send!(sender, Action::InitEpisode(episode.rowid()));
        // Refresh background views to match the normal/greyout title state
        send!(sender, Action::RefreshEpisodesViewBGR);
        Ok(())
    }

    fn init_info(&self, episode: &EpisodeWidgetModel) {
        // Set the title label state.
        self.set_title(episode);

        // Set the date label.
        self.set_date(episode.epoch());

        // Set the duration label.
        self.set_duration(episode.duration());

        // Set the total_size label.
        self.set_size(episode.length())
    }

    fn set_title(&self, episode: &EpisodeWidgetModel) {
        self.title.set_text(episode.title());

        if episode.played().is_some() {
            self.title.add_css_class("dim-label");
            self.played_checkmark.set_visible(true);
        } else {
            self.title.remove_css_class("dim-label");
            self.played_checkmark.set_visible(false);
        }
    }

    // Set the date label of the episode widget.
    fn set_date(&self, epoch: i32) {
        let now: DateTime<Utc> = Utc::now();

        let ts = Utc.timestamp_opt(i64::from(epoch), 0).unwrap();

        // If the episode is from a different year, print year as well
        if now.year() != ts.year() {
            self.date.set_text(ts.format("%e %b %Y").to_string().trim());
        // Else omit the year from the label
        } else {
            self.date.set_text(ts.format("%e %b").to_string().trim());
        }
    }

    // Set the duration label of the episode widget.
    fn set_duration(&self, seconds: Option<i32>) {
        // If length is provided
        if let Some(s) = seconds {
            // Convert seconds to minutes
            let minutes = chrono::Duration::seconds(s.into()).num_minutes();
            // If the length is 1 or more minutes
            if minutes != 0 {
                // Set the label and show them.
                self.duration
                    .set_text(&i18n_f("{} min", &[&minutes.to_string()]));
                self.duration.set_visible(true);
                self.separator1.set_visible(true);
                return;
            }
        }

        // Else hide the labels
        self.separator1.set_visible(false);
        self.duration.set_visible(false);
    }

    // Set the size label of the episode widget.
    fn set_size(&self, bytes: Option<i32>) {
        // Convert the bytes to a String label
        let size = bytes.and_then(|s| {
            if s == 0 {
                None
            } else {
                s.file_size(SIZE_OPTS.clone()).ok()
            }
        });

        if let Some(s) = size {
            self.total_size.set_text(&s);
            self.total_size.set_visible(true);
            self.separator2.set_visible(true);
        } else {
            self.total_size.set_visible(false);
            self.separator2.set_visible(false);
        }
    }
}
fn on_download_clicked(ep: &EpisodeWidgetModel, sender: &Sender<Action>) -> Result<()> {
    let pd = dbqueries::get_podcast_from_id(ep.show_id())?;
    let download_dir = get_download_dir(pd.title())?;

    // Start a new download.
    manager::add(ep.rowid(), download_dir)?;

    // Update Views
    send!(sender, Action::RefreshEpisodesViewBGR);
    Ok(())
}

// Setup a callback that will update the progress bar.
#[inline]
fn update_progressbar_callback(
    widget: &EpisodeWidgetPriv,
    prog: &Arc<Mutex<manager::Progress>>,
    episode_rowid: i32,
) {
    let callback = clone!(@weak widget, @strong prog => @default-return glib::Continue(false), move || {
        progress_bar_helper(&widget, &prog, episode_rowid)
            .unwrap_or(glib::Continue(false))
    });
    glib::timeout_add_local(Duration::from_millis(100), callback);
}

fn progress_bar_helper(
    widget: &EpisodeWidgetPriv,
    prog: &Arc<Mutex<manager::Progress>>,
    episode_rowid: i32,
) -> Result<glib::Continue> {
    let (fraction, downloaded, cancel) = match prog.try_lock() {
        Ok(guard) => (
            guard.get_fraction(),
            guard.get_downloaded(),
            guard.should_cancel(),
        ),
        Err(TryLockError::WouldBlock) => return Ok(glib::Continue(true)),
        Err(TryLockError::Poisoned(_)) => return Err(anyhow!("Progress Mutex is poisoned")),
    };

    // Update the progress_bar.
    if (0.0..=1.0).contains(&fraction) && (!fraction.is_nan()) {
        // Update local_size label
        let size = downloaded
            .file_size(SIZE_OPTS.clone())
            .map_err(|err| anyhow!("{}", err))?;

        widget.update_progress(&size, fraction);
    }

    // Check if the download is still active
    let active = match manager::ACTIVE_DOWNLOADS.read() {
        Ok(guard) => guard.contains_key(&episode_rowid),
        Err(_) => return Err(anyhow!("Failed to get a lock on the mutex.")),
    };

    if (fraction >= 1.0) && (!fraction.is_nan()) {
        Ok(glib::Continue(false))
    } else if !active || cancel {
        // if the total size is not a number, hide it
        if widget
            .total_size
            .text()
            .trim_end_matches(" MB")
            .parse::<i32>()
            .is_err()
        {
            widget.total_size.set_visible(false);
        }
        Ok(glib::Continue(false))
    } else {
        Ok(glib::Continue(true))
    }
}

// Setup a callback that will update the total_size label
// with the http ContentLength header number rather than
// relying to the RSS feed.
#[inline]
fn update_total_size_callback(widget: &EpisodeWidgetPriv, prog: &Arc<Mutex<manager::Progress>>) {
    let callback = clone!(@strong prog, @weak widget => @default-return glib::Continue(false), move || {
        total_size_helper(&widget, &prog).unwrap_or(glib::Continue(true))
    });
    glib::timeout_add_local(Duration::from_millis(100), callback);
}

fn total_size_helper(
    widget: &EpisodeWidgetPriv,
    prog: &Arc<Mutex<manager::Progress>>,
) -> Result<glib::Continue> {
    // Get the total_bytes.
    let total_bytes = match prog.try_lock() {
        Ok(guard) => guard.get_size(),
        Err(TryLockError::WouldBlock) => return Ok(glib::Continue(true)),
        Err(TryLockError::Poisoned(_)) => return Err(anyhow!("Progress Mutex is poisoned")),
    };

    debug!("Total Size: {}", total_bytes);
    if total_bytes != 0 {
        // Update the total_size label
        widget.set_size(Some(total_bytes as i32));

        // Do not call again the callback
        Ok(glib::Continue(false))
    } else {
        Ok(glib::Continue(true))
    }
}

#[glib::object_subclass]
impl ObjectSubclass for EpisodeWidgetPriv {
    const NAME: &'static str = "PdEpisode";
    type Type = EpisodeWidget;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for EpisodeWidgetPriv {}
impl ObjectImpl for EpisodeWidgetPriv {}
impl BoxImpl for EpisodeWidgetPriv {}

glib::wrapper! {
    pub struct EpisodeWidget(ObjectSubclass<EpisodeWidgetPriv>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl EpisodeWidget {
    pub(crate) fn new(sender: &Sender<Action>, episode: &EpisodeWidgetModel) -> Self {
        let widget = Self::default();
        widget.imp().init(sender, episode);
        widget
    }
}

impl Default for EpisodeWidget {
    fn default() -> Self {
        let widget: Self = glib::Object::new();
        widget
    }
}

// fn on_delete_bttn_clicked(episode_id: i32) -> Result<()> {
//     let mut ep = dbqueries::get_episode_from_rowid(episode_id)?.into();
//     delete_local_content(&mut ep).map_err(From::from).map(|_| ())
// }

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
use anyhow::Result;
use async_channel::Sender;
use chrono::prelude::*;
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::app::Action;
use crate::i18n::i18n_f;
use crate::manager;
use crate::widgets::DownloadProgressBar;
use podcasts_data::dbqueries;
use podcasts_data::utils::get_download_dir;
use podcasts_data::EpisodeId;
use podcasts_data::EpisodeWidgetModel;

static SIZE_OPTS: Lazy<humansize::FormatSizeOptions> = Lazy::new(|| {
    // Declare a custom humansize option struct
    // See: https://docs.rs/humansize/2.1.3/humansize/struct.FormatSizeOptions.html
    humansize::FormatSizeOptions::from(humansize::WINDOWS).decimal_places(0)
});

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/episode_widget.ui")]
pub struct EpisodeWidgetPriv {
    #[template_child]
    progressbar: TemplateChild<DownloadProgressBar>,

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
    #[template_child]
    text_only: TemplateChild<gtk::Button>,
}

impl EpisodeWidgetPriv {
    pub(crate) fn init(&self, sender: &Sender<Action>, episode: EpisodeWidgetModel) {
        crate::MAINCONTEXT.spawn_local_with_priority(
            glib::source::Priority::LOW,
            clone!(
                #[weak(rename_to = this)]
                self,
                #[strong]
                sender,
                async move {
                    let id = episode.id();
                    this.init_info(&episode);
                    if episode.uri().is_none() {
                        this.state_no_uri(id);
                        return;
                    }

                    this.init_progressbar(id);
                    this.init_buttons(&sender, id);
                    if let Err(err) = this.determine_buttons_state(&episode) {
                        error!("Error: {}", err);
                    }
                }
            ),
        );
    }

    // Rare case when an episode does not have
    // any audio files attached as enclosure tags.
    fn state_no_uri(&self, id: EpisodeId) {
        self.cancel.set_visible(false);
        self.play.set_visible(false);

        self.local_size.set_visible(false);
        self.size_separator.set_visible(false);
        self.download.set_visible(false);
        self.text_only.set_visible(true);
        self.text_only.set_action_name(Some("app.go-to-episode"));
        self.text_only
            .set_action_target_value(Some(&id.0.to_variant()));
    }

    // InProgress State:
    //   * Show ProgressBar and Cancel Button.
    //   * Show `total_size`, `local_size` labels and `size_separator`.
    //   * Hide Download and Play Buttons
    fn state_prog(&self) {
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
        self.cancel.set_visible(false);
        self.download.set_visible(false);
        self.local_size.set_visible(false);
        self.size_separator.set_visible(false);

        self.total_size.set_visible(true);
        self.play.set_visible(true);
    }

    // NotDownloaded State:
    //   * Hide ProgressBar and Cancel, Play Buttons.
    //   * Hide `local_size` labels and `size_separator`.
    //   * Show Download Button
    //   * Determine `total_size` label state (Comes from `episode.lenght`).
    fn state_download(&self) {
        self.cancel.set_visible(false);
        self.play.set_visible(false);

        self.local_size.set_visible(false);
        self.size_separator.set_visible(false);

        self.download.set_visible(true);
    }

    /// Change the state of the `EpisodeWidget`.
    ///
    /// Function Flowchart:
    ///
    /// -------------------       --------------
    /// | Does the Episode|  YES  |   State:   |
    /// |   not have a    | ----> |   NoUri    |
    /// | download link?  |       |            |
    /// -------------------       --------------
    ///         |
    ///         | NO
    ///         |
    ///        \_/
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
    /// |  NotDownloaded  |
    /// -------------------
    fn determine_buttons_state(&self, episode: &EpisodeWidgetModel) -> Result<()> {
        let is_downloading = self.progressbar.check_if_downloading()?;
        if is_downloading {
            // State InProgress
            self.state_prog();
        } else if episode.local_uri().is_some() {
            // State: Playable
            self.state_playable();
        } else {
            // State: NotDownloaded
            self.state_download();
        }
        Ok(())
    }

    fn init_info(&self, episode: &EpisodeWidgetModel) {
        self.set_title(episode);
        self.set_date(episode.epoch());
        self.set_duration(episode.duration());
        self.set_size(episode.length());
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
    fn set_date(&self, epoch: NaiveDateTime) {
        let now: DateTime<Local> = Local::now();
        let ts = DateTime::<Local>::from(epoch.and_utc());

        // If the episode is from a different year, print year as well
        if now.year() != ts.year() {
            self.date.set_text(
                ts.format_localized("%e %b %Y", *crate::CHRONO_LOCALE)
                    .to_string()
                    .trim(),
            );
        // Else omit the year from the label
        } else {
            self.date.set_text(
                ts.format_localized("%e %b", *crate::CHRONO_LOCALE)
                    .to_string()
                    .trim(),
            );
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
            if s <= 0 {
                None
            } else {
                Some(humansize::format_size(s as u32, *SIZE_OPTS))
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

    fn init_progressbar(&self, id: EpisodeId) {
        self.progressbar.init(id);

        self.progressbar.connect_state_change(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                if let Err(err) = dbqueries::get_episode_widget_from_id(id)
                    .map(|ep| this.determine_buttons_state(&ep))
                {
                    error!("Could not get episode info: {err}");
                }
            }
        ));

        self.progressbar
            .bind_property("local_size", &*self.local_size, "label")
            .transform_to(move |_, downloaded: u64| {
                Some(humansize::format_size(downloaded, *SIZE_OPTS))
            })
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();

        self.progressbar.connect_total_size_notify(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                // try_from should handle NaN case
                this.set_size(i32::try_from(this.progressbar.total_size()).ok());
            }
        ));
    }

    fn init_buttons(&self, sender: &Sender<Action>, id: EpisodeId) {
        self.cancel.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                if let Err(e) = this.progressbar.cancel() {
                    error!("failed to cancel download {e}");
                }
            }
        ));

        self.play.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[strong]
            sender,
            move |_| {
                if let Ok(episode) = dbqueries::get_episode_widget_from_id(id) {
                    // Grey out the title
                    this.set_title(&episode);
                    // Play the episode
                    send_blocking!(sender, Action::InitEpisode(episode.id()));
                    // Refresh background views to match the normal/greyout title state
                    send_blocking!(sender, Action::RefreshEpisodesViewBGR);
                }
            }
        ));

        self.download.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[strong]
            sender,
            move |dl| {
                if let Ok(ep) = dbqueries::get_episode_widget_from_id(id) {
                    let result = on_download_clicked(&ep, &sender).and_then(|_| {
                        info!("Download started successfully.");
                        this.determine_buttons_state(&ep)
                    });
                    if let Err(err) = result {
                        error!("Failed to start download {err}");
                    } else {
                        this.progressbar.grab_focus();
                    }
                }

                // Restore sensitivity after operations above complete
                dl.set_sensitive(true);
            }
        ));
    }
}
fn on_download_clicked(ep: &EpisodeWidgetModel, sender: &Sender<Action>) -> Result<()> {
    let pd = dbqueries::get_podcast_from_id(ep.show_id())?;
    let download_dir = get_download_dir(pd.title())?;

    // Start a new download.
    manager::add(sender.clone(), ep.id(), download_dir)?;
    // Update Views
    send_blocking!(sender, Action::RefreshEpisodesViewBGR);
    Ok(())
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
    pub(crate) fn new(sender: &Sender<Action>, episode: EpisodeWidgetModel) -> Self {
        let widget = Self::default();
        widget.init(sender, episode);
        widget
    }

    pub(crate) fn init(&self, sender: &Sender<Action>, episode: EpisodeWidgetModel) {
        self.imp().init(sender, episode);
    }
}

impl Default for EpisodeWidget {
    fn default() -> Self {
        let widget: Self = glib::Object::new();
        widget
    }
}

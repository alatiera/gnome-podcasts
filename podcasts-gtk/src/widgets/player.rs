// player.rs
//
// Copyright 2018 Jordan Petridis <jpetridis@gnome.org>
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

use adw::prelude::*;
use anyhow::Result;
use async_channel::Sender;
use chrono::prelude::*;
use fragile::Fragile;
use gio::File;
use glib::clone;
use glib::{SignalHandlerId, WeakRef};
use gst::ClockTime;
use gtk::{gio, glib};
use mpris_server::{Metadata, PlaybackStatus, Player};
use once_cell::sync::Lazy;
use std::cell::{RefCell, RefMut};
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;
use url::Url;

use crate::app::Action;
use crate::config::APP_ID;
use crate::download_covers::load_widget_texture;
use crate::i18n::i18n;
use podcasts_data::{
    EpisodeId, EpisodeModel, EpisodeWidgetModel, ShowCoverModel, ShowId, USER_AGENT, dbqueries,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum SeekDirection {
    Backwards,
    Forward,
}

pub(crate) trait PlayerExt {
    fn play(&self);
    fn pause(&mut self);
    fn toggle_pause(&mut self);
    fn stop(&mut self);
    fn seek(&self, offset: ClockTime, direction: SeekDirection) -> Option<()>;
    fn fast_forward(&self);
    fn rewind(&self);
    fn set_playback_rate(&self, _: f64);
}

#[derive(Debug, Clone)]
struct PlayerInfo {
    show: gtk::Label,
    episode: gtk::Label,
    cover: gtk::Image,
    show_small: gtk::Label,
    episode_small: gtk::Label,
    cover_small: gtk::Image,
    mpris: Option<Rc<Player>>,
    restore_position: Option<i32>,
    finished_restore: bool,
    ep: Option<EpisodeWidgetModel>,
    episode_id: RefCell<Option<EpisodeId>>,
}

impl PlayerInfo {
    fn create_bindings(&self) {
        self.show
            .bind_property("label", &self.show_small, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        self.episode
            .bind_property("label", &self.episode_small, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
    }

    // FIXME: create a Diesel Model of the joined episode and podcast query instead
    fn init(
        &mut self,
        sender: &Sender<Action>,
        episode: &EpisodeWidgetModel,
        podcast: &ShowCoverModel,
    ) {
        self.ep = Some(episode.clone());
        self.episode_id.replace(Some(episode.id()));
        self.set_cover_image(podcast);
        self.set_show_title(podcast);
        self.set_episode_title(episode);

        let mut metadata = Metadata::new();
        metadata.set_artist(Some(vec![podcast.title().to_string()]));
        metadata.set_title(Some(episode.title().to_string()));
        metadata.set_length(
            episode
                .duration()
                .map(|s| mpris_server::Time::from_secs(s as i64)),
        );

        // Set the cover if it is already cached.
        let art_path = crate::download_covers::determin_cover_path(podcast, None);
        if art_path.exists() {
            metadata.set_art_url(Url::from_file_path(art_path).ok());
        } else {
            // If the cover art doesn't already exist, download it
            let sender = sender.clone();
            let podcast = podcast.clone();
            crate::RUNTIME.spawn(async move {
                let id = podcast.id();
                if let Err(err) = crate::download_covers::just_download(&podcast).await {
                    error!("Cover download failed {err}");
                    send!(sender, Action::UpdateMprisCover(id, false));
                } else {
                    send!(sender, Action::UpdateMprisCover(id, true));
                }
            });
        }

        if let Some(mpris) = self.mpris.as_ref() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                glib::source::Priority::LOW,
                clone!(
                    #[weak]
                    mpris,
                    async move {
                        if let Err(err) = mpris.set_metadata(metadata).await {
                            warn!("Failed to set MPRIS metadata: {err:?}");
                        }
                        if let Err(err) = mpris.set_can_pause(true).await {
                            warn!("Failed to set MPRIS pause capability: {err:?}");
                        }
                        if let Err(err) = mpris.set_can_play(true).await {
                            warn!("Failed to set MPRIS play capability: {err:?}");
                        }
                        if let Err(err) = mpris.set_can_seek(true).await {
                            warn!("Failed to set MPRIS seek capability: {err:?}");
                        }
                    }
                ),
            );
        }
    }

    // hook for when the async download finished
    fn update_mpris_cover(&self, show_id: ShowId, dl_success: bool) -> Result<()> {
        if let Some(ep) = self.ep.as_ref() {
            if ep.show_id() != show_id {
                // Download took too long, we are no longer on the same show.
                return Ok(());
            }
        }

        if let Some(mpris) = self.mpris.as_ref() {
            let pd = dbqueries::get_podcast_cover_from_id(show_id)?;
            let mut metadata = mpris.metadata().clone();
            if dl_success {
                let art_path = crate::download_covers::determin_cover_path(&pd, None);
                if art_path.exists() {
                    metadata.set_art_url(Url::from_file_path(art_path).ok());
                    crate::MAINCONTEXT.spawn_local_with_priority(
                        glib::source::Priority::LOW,
                        clone!(
                            #[weak]
                            mpris,
                            async move {
                                if let Err(err) = mpris.set_metadata(metadata).await {
                                    error!("failed to update mpris metadata {err}");
                                }
                            }
                        ),
                    );
                    return Ok(());
                } else {
                    error!(
                        "cover does not exist after successful download? {}",
                        art_path.display()
                    );
                }
            }
            // Fallback to web url, it could still work,
            // because of different http agent or no disk space.
            metadata.set_art_url(pd.image_uri());
            crate::MAINCONTEXT.spawn_local_with_priority(
                glib::source::Priority::LOW,
                clone!(
                    #[weak]
                    mpris,
                    async move {
                        if let Err(err) = mpris.set_metadata(metadata).await {
                            error!("failed to update mpris metadata {err}");
                        }
                    }
                ),
            );
        }
        Ok(())
    }

    fn set_episode_title(&self, episode: &EpisodeWidgetModel) {
        self.episode.set_text(episode.title());
        self.episode.set_tooltip_text(Some(episode.title()));
    }

    fn set_show_title(&self, show: &ShowCoverModel) {
        self.show.set_text(show.title());
        self.show.set_tooltip_text(Some(show.title()));
    }

    fn set_cover_image(&self, show: &ShowCoverModel) {
        load_widget_texture(&self.cover, show.id(), crate::Thumb64);
        load_widget_texture(&self.cover_small, show.id(), crate::Thumb64);
    }

    fn update_mpris_position(&self, position: Position) -> Option<()> {
        let time = mpris_server::Time::from_secs(position.seconds() as i64);
        self.mpris.as_ref()?.set_position(time);
        Some(())
    }
}

#[derive(Debug, Clone)]
struct PlayerTimes {
    progressed: gtk::Label,
    duration: gtk::Label,
    slider: gtk::Scale,
    slider_update: Rc<SignalHandlerId>,
    progress_bar: gtk::ProgressBar,
}

#[derive(Debug, Clone, Copy)]
struct Duration(ClockTime);

impl Deref for Duration {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
struct Position(ClockTime);

impl Deref for Position {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PlayerTimes {
    /// Update the duration `gtk::Label` and the max range of the `gtk::SclaeBar`.
    pub(crate) fn on_duration_changed(&self, duration: Duration) {
        let seconds = duration.seconds();

        self.slider.block_signal(&self.slider_update);
        self.slider.set_range(0.0, seconds as f64);
        self.slider.unblock_signal(&self.slider_update);

        self.duration.set_text(&format_duration(seconds as u32));

        self.update_progress_bar();
    }

    /// Update the `gtk::Scale` bar when the pipeline position is changed.
    pub(crate) fn on_position_updated(&self, position: Position) {
        let seconds = position.seconds();

        self.slider.block_signal(&self.slider_update);
        self.slider.set_value(seconds as f64);
        self.slider.unblock_signal(&self.slider_update);

        self.progressed.set_text(&format_duration(seconds as u32));

        self.update_progress_bar();
    }

    fn update_progress_bar(&self) {
        let fraction = self.slider.value() / self.slider.adjustment().upper();
        self.progress_bar.set_fraction(fraction);
    }
}

fn format_duration(seconds: u32) -> String {
    let time = NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0);
    if time.is_none() {
        return "-".to_string();
    }
    let time = time.unwrap();

    if seconds >= 3600 {
        time.format("%T").to_string()
    } else {
        time.format("%M:%S").to_string()
    }
}

#[derive(Debug, Clone)]
struct PlayerRate {
    action: gio::SimpleAction,
    btn: gtk::MenuButton,
}

impl PlayerRate {
    fn new() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/player_rate.ui");

        // This needs to be a string to work with GMenuModel
        let variant_type = glib::VariantTy::new("s").expect("Could not parse variant type");
        let action =
            gio::SimpleAction::new_stateful("set", Some(variant_type), &"1.00".to_variant());
        let btn: gtk::MenuButton = builder.object("rate_button").unwrap();

        PlayerRate { action, btn }
    }

    fn connect_signals(&self, widget: &Rc<RefCell<PlayerWidget>>) {
        let group = gio::SimpleActionGroup::new();
        self.action.connect_activate(clone!(
            #[weak]
            widget,
            move |action, rate_v| {
                let variant = rate_v.unwrap();
                action.set_state(variant);
                let rate = variant
                    .get::<String>()
                    .expect("Could not get rate from variant")
                    .parse::<f64>()
                    .expect("Could not parse float from variant string");
                widget.borrow().on_rate_changed(rate);
            }
        ));
        group.add_action(&self.action);
        widget
            .borrow()
            .container
            .insert_action_group("rate", Some(&group));
        widget
            .borrow()
            .sheet
            .sheet
            .insert_action_group("rate", Some(&group));
    }
}

#[derive(Debug, Clone)]
struct PlayerControls {
    play: gtk::Button,
    pause: gtk::Button,
    play_pause_big: gtk::Stack,
    play_small: gtk::Button,
    pause_small: gtk::Button,
    play_pause_small: gtk::Stack,
    forward: gtk::Button,
    rewind: gtk::Button,
    last_pause: RefCell<Option<DateTime<Local>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerSheet {
    pub(crate) sheet: adw::Bin,
    cover: gtk::Image,
    play_pause: gtk::Stack,
    play: gtk::Button,
    pause: gtk::Button,
    duration: gtk::Label,
    progressed: gtk::Label,
    slider: gtk::Scale,
    forward: gtk::Button,
    rewind: gtk::Button,
    rate: PlayerRate,
    show: gtk::Label,
    episode: gtk::Label,
    go_to_episode: gtk::Button,
}

impl PlayerSheet {
    fn new(rate: PlayerRate) -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/player_sheet.ui");
        let sheet = builder.object("sheet").unwrap();

        let cover = builder.object("cover").unwrap();
        let play_pause = builder.object("play_pause").unwrap();
        let play = builder.object("play").unwrap();
        let pause = builder.object("pause").unwrap();
        let duration = builder.object("duration").unwrap();
        let progressed = builder.object("progressed").unwrap();
        let slider = builder.object("slider").unwrap();
        let rewind = builder.object("rewind").unwrap();
        let forward = builder.object("forward").unwrap();
        let rate_container: adw::Bin = builder.object("rate_container").unwrap();
        let show = builder.object("show_label").unwrap();
        let episode = builder.object("episode_label").unwrap();
        let go_to_episode: gtk::Button = builder.object("go_to_episode").unwrap();

        rate_container.set_child(Some(&rate.btn));

        go_to_episode.connect_clicked(|button| {
            let app = gio::Application::default()
                .expect("Could not get default application")
                .downcast::<gtk::Application>()
                .unwrap();

            app.activate_action("go-to-episode", button.action_target_value().as_ref());
            let window = app.active_window().expect("No active window");
            if let Err(e) = window.activate_action("win.close-bottom-sheet", None) {
                warn!("Failed to win.close-bottom-sheet {e}");
            }
        });

        PlayerSheet {
            sheet,
            cover,
            play_pause,
            play,
            pause,
            duration,
            progressed,
            slider,
            forward,
            rewind,
            rate,
            show,
            episode,
            go_to_episode,
        }
    }

    fn initialize_episode(&self, episode: &EpisodeWidgetModel, show: &ShowCoverModel) {
        self.episode.set_text(episode.title());
        self.show.set_text(show.title());
        load_widget_texture(&self.cover, show.id(), crate::Thumb256);
        self.go_to_episode
            .set_action_target(Some::<gtk::glib::Variant>(episode.id().0.into()));
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerWidget {
    pub(crate) container: gtk::Box,
    player: gst_play::Play,
    player_signals: gst_play::PlaySignalAdapter,
    controls: PlayerControls,
    pub(crate) sheet: PlayerSheet,
    full: gtk::Box,
    small: gtk::Box,
    stack: gtk::Stack,
    timer: PlayerTimes,
    info: PlayerInfo,
    rate: PlayerRate,
    sender: Option<Sender<Action>>,
}

impl Default for PlayerWidget {
    fn default() -> Self {
        let player = gst_play::Play::default();
        let player_signals = gst_play::PlaySignalAdapter::new(&player);

        // A few podcasts have a video track of the thumbnail, which GStreamer displays in a new
        // window. Make sure it doesn't do that.
        player.set_video_track_enabled(false);
        let mpris = crate::RUNTIME.block_on(async move {
            let mpris_result = Player::builder(APP_ID)
                .identity(i18n("Podcasts"))
                .desktop_entry(APP_ID)
                .can_raise(true)
                .can_pause(false)
                .can_play(false)
                .can_seek(false)
                .can_set_fullscreen(false)
                .can_go_next(false)
                .can_go_previous(false)
                .build()
                .await;
            match mpris_result {
                Err(e) => {
                    error!("mpris initialization: {e}");
                    None
                }
                Ok(mpris) => Some(Rc::new(mpris)),
            }
        });

        if let Some(mpris) = mpris.as_ref() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                glib::source::Priority::LOW,
                clone!(
                    #[weak]
                    mpris,
                    async move {
                        let task = mpris.run();
                        task.await;
                    }
                ),
            );
        }

        let mut config = player.config();
        config.set_user_agent(USER_AGENT);
        config.set_position_update_interval(250);
        player.set_config(config).unwrap();

        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/player_toolbar.ui");

        let play = builder.object("play_button").unwrap();
        let pause = builder.object("pause_button").unwrap();
        let play_small = builder.object("play_button_small").unwrap();
        let pause_small = builder.object("pause_button_small").unwrap();
        let forward: gtk::Button = builder.object("ff_button").unwrap();
        let rewind: gtk::Button = builder.object("rewind_button").unwrap();
        let play_pause_small = builder.object("play_pause_small").unwrap();
        let play_pause_big = builder.object("play_pause_big").unwrap();

        let controls = PlayerControls {
            play,
            pause,
            play_pause_big,
            play_small,
            pause_small,
            play_pause_small,
            forward,
            rewind,
            last_pause: RefCell::new(None),
        };

        let progressed = builder.object("progress_time_label").unwrap();
        let duration = builder.object("total_duration_label").unwrap();
        let slider: gtk::Scale = builder.object("seek").unwrap();
        slider.set_range(0.0, 1.0);
        let player_weak = player.downgrade();
        let slider_update = Rc::new(Self::connect_update_slider(&slider, player_weak));
        let progress_bar = builder.object("progress_bar").unwrap();
        let timer = PlayerTimes {
            progressed,
            duration,
            slider,
            slider_update,
            progress_bar,
        };

        let show = builder.object("show_label").unwrap();
        let episode = builder.object("episode_label").unwrap();
        let cover = builder.object("show_cover").unwrap();
        let show_small = builder.object("show_label_small").unwrap();
        let episode_small = builder.object("episode_label_small").unwrap();
        let cover_small = builder.object("show_cover_small").unwrap();
        let ep = None;
        let info = PlayerInfo {
            mpris,
            show,
            ep,
            episode,
            cover,
            show_small,
            episode_small,
            cover_small,
            restore_position: None,
            finished_restore: false,
            episode_id: RefCell::new(None),
        };
        info.create_bindings();

        let dialog_rate = PlayerRate::new();
        let dialog = PlayerSheet::new(dialog_rate);

        let container = builder.object("container").unwrap();
        let full: gtk::Box = builder.object("full").unwrap();
        let small: gtk::Box = builder.object("small").unwrap();
        let stack = builder.object("stack").unwrap();

        let rate = PlayerRate::new();
        full.append(&rate.btn);

        PlayerWidget {
            player,
            player_signals,
            container,
            controls,
            sheet: dialog,
            full,
            small,
            stack,
            timer,
            info,
            rate,
            sender: None,
        }
    }
}

#[derive(PartialEq)]
pub enum StreamMode {
    LocalOnly,
    StreamOnly,
    StreamFallback,
}

impl PlayerWidget {
    fn on_rate_changed(&self, rate: f64) {
        self.set_playback_rate(rate);
        self.rate.btn.set_label(&format!("{:.2}×", rate));
        self.sheet.rate.btn.set_label(&format!("{:.2}×", rate));
    }

    fn reveal(&self) {
        self.container.set_visible(true);
    }

    pub(crate) fn initialize_episode(
        &mut self,
        sender: &Sender<Action>,
        id: EpisodeId,
        stream: StreamMode,
        second: Option<i32>,
    ) -> Result<()> {
        let ep = dbqueries::get_episode_widget_from_id(id)?;
        let pd = dbqueries::get_podcast_cover_from_id(ep.show_id())?;

        self.sheet.initialize_episode(&ep, &pd);

        self.info.restore_position = second.or_else(|| {
            let episode_position = ep.play_position();
            if episode_position == 0 {
                None
            } else {
                Some(episode_position)
            }
        });
        self.info.finished_restore = false;
        self.info.init(sender, &ep, &pd);

        if stream == StreamMode::StreamOnly {
            if let Some(uri) = ep.uri() {
                self.init_uri(uri, second);
                return Ok(());
            } else {
                error!("No uri for episode");
            }
        // Currently that will always be the case since the play button is
        // only shown if the file is downloaded
        } else if let Some(ref path) = ep.local_uri() {
            if Path::new(path).exists() {
                // path is an absolute fs path ex. "foo/bar/baz".
                // Convert it so it will have a "file:///"
                // FIXME: convert it properly
                let uri = File::for_path(path).uri();
                self.init_uri(uri.as_str(), second);
                return Ok(());
            } else {
                error!("failed to create path for episode {:#?}", ep);
            }
        } else if stream == StreamMode::StreamFallback {
            if let Some(uri) = ep.uri() {
                self.init_uri(uri, second);
                return Ok(());
            } else {
                error!("No uri for episode");
            }
        } else {
            error!("Episode not downloaded yet.");
        }

        Ok(())
    }

    fn init_uri(&mut self, uri: &str, second: Option<i32>) {
        // If it's not the same file load the uri, otherwise just unpause
        if self.player.uri().is_none_or(|s| s != uri) {
            self.player.set_uri(Some(uri));
        } else if second.is_some() {
            // force a jump now if already playing and a jump is given
            self.restore_play_position();
        } else {
            // just unpause, no restore required
            self.info.finished_restore = true;
        }
        // play the file
        self.play();
    }

    fn connect_update_slider(
        slider: &gtk::Scale,
        player: WeakRef<gst_play::Play>,
    ) -> SignalHandlerId {
        slider.connect_value_changed(move |slider| {
            let player = match player.upgrade() {
                Some(p) => p,
                None => return,
            };

            let value = slider.value() as u64;
            player.seek(ClockTime::from_seconds(value));
        })
    }

    pub fn update_mpris_cover(&self, show_id: ShowId, dl_success: bool) -> Result<()> {
        self.info.update_mpris_cover(show_id, dl_success)
    }

    fn smart_rewind(&self) -> Option<()> {
        static LAST_KNOWN_EPISODE: Lazy<Mutex<Option<EpisodeId>>> = Lazy::new(|| Mutex::new(None));

        // Figure out the time delta, in seconds, between the last pause and now
        let now = Local::now();
        let last: &Option<DateTime<_>> = &*self.controls.last_pause.borrow();
        let delta = (now - (*last)?).num_seconds();

        // Get interval passed in the gst stream
        let seconds_passed = self.player.position()?.seconds();
        // get the last known episode id
        let mut last = LAST_KNOWN_EPISODE.lock().unwrap();
        // get the current playing episode id
        let current_id = *self.info.episode_id.borrow();
        // Only rewind on pause if the stream position is passed a certain point,
        // and the player has been paused for more than a minute,
        // and the episode id is the same
        if seconds_passed >= 90 && delta >= 60 && current_id == *last {
            self.seek(ClockTime::from_seconds(5), SeekDirection::Backwards);
        }

        // Set the last knows episode to the current one
        *last = current_id;

        Some(())
    }

    /// Seek to the `play_position` stored in the episode.
    /// Returns Some(()) if the restore was successful and None otherwise.
    fn restore_play_position(&self) -> Option<()> {
        let pos = self.info.restore_position;
        if let Some(pos) = pos {
            let s: u64 = pos.try_into().ok()?;
            self.player.seek(ClockTime::from_seconds(s));
            Some(())
        } else {
            None
        }
    }

    pub fn set_small(&self, small: bool) {
        if small {
            self.stack.set_visible_child(&self.small);
        } else {
            self.stack.set_visible_child(&self.full);
        }
    }
}

impl PlayerExt for PlayerWidget {
    fn play(&self) {
        self.sheet.play_pause.set_visible_child(&self.sheet.pause);

        self.reveal();

        self.controls
            .play_pause_big
            .set_visible_child(&self.controls.pause);
        self.controls
            .play_pause_small
            .set_visible_child(&self.controls.pause_small);

        self.smart_rewind();
        self.player.play();
        if let Some(mpris) = self.info.mpris.as_ref() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                glib::source::Priority::LOW,
                clone!(
                    #[weak]
                    mpris,
                    async move {
                        if let Err(err) = mpris.set_playback_status(PlaybackStatus::Playing).await {
                            warn!("Failed to set MPRIS playback status: {err:?}");
                        }
                    }
                ),
            );
        }
        if let Some(sender) = &self.sender {
            send_blocking!(sender, Action::InhibitSuspend);
        }
    }

    fn pause(&mut self) {
        self.sheet.play_pause.set_visible_child(&self.sheet.play);

        self.controls
            .play_pause_big
            .set_visible_child(&self.controls.play);
        self.controls
            .play_pause_small
            .set_visible_child(&self.controls.play_small);

        self.player.pause();
        if let Some(mpris) = self.info.mpris.as_ref() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                glib::source::Priority::LOW,
                clone!(
                    #[weak]
                    mpris,
                    async move {
                        if let Err(err) = mpris.set_playback_status(PlaybackStatus::Paused).await {
                            warn!("Failed to set MPRIS playback status: {err:?}");
                        }
                    }
                ),
            );
        }
        if let Some(sender) = &self.sender {
            send_blocking!(sender, Action::UninhibitSuspend);
        }

        self.controls.last_pause.replace(Some(Local::now()));
        let pos = self.player.position();
        self.info.ep.as_mut().map(|ep| {
            ep.set_play_position(pos.and_then(|s| s.seconds().try_into().ok()).unwrap_or(0))
        });
    }

    fn stop(&mut self) {
        // hide pause buttons and restore focus for accessibility
        let is_focus = self.controls.pause.is_focus();
        self.controls
            .play_pause_big
            .set_visible_child(&self.controls.play);
        if is_focus {
            self.controls.play.grab_focus();
        }

        let is_focus = self.controls.pause_small.is_focus();
        self.controls
            .play_pause_small
            .set_visible_child(&self.controls.play_small);
        if is_focus {
            self.controls.play_small.grab_focus();
        }

        let is_focus = self.sheet.pause.is_focus();
        self.sheet.play_pause.set_visible_child(&self.sheet.play);
        if is_focus {
            self.sheet.play.grab_focus();
        }

        self.info.ep = None;
        self.info.restore_position = None;
        self.player.stop();
        if let Some(mpris) = self.info.mpris.as_ref() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                glib::source::Priority::LOW,
                clone!(
                    #[weak]
                    mpris,
                    async move {
                        if let Err(err) = mpris.set_playback_status(PlaybackStatus::Paused).await {
                            warn!("Failed to set MPRIS playback status: {err:?}");
                        }
                    }
                ),
            );
        }

        // Reset the slider bar to the start

        self.timer
            .on_position_updated(Position(ClockTime::from_seconds(0)));
        if let Some(sender) = &self.sender {
            send_blocking!(sender, Action::UninhibitSuspend);
        }
    }

    fn toggle_pause(&mut self) {
        if let Some(mpris) = self.info.mpris.as_ref() {
            match mpris.playback_status() {
                PlaybackStatus::Paused => self.play(),
                PlaybackStatus::Stopped => self.play(),
                _ => self.pause(),
            };
        }
    }

    // Adapted from https://github.com/philn/glide/blob/b52a65d99daeab0b487f79a0e1ccfad0cd433e22/src/player_context.rs#L219-L245
    fn seek(&self, offset: ClockTime, direction: SeekDirection) -> Option<()> {
        // How far into the podcast we are
        let position = self.player.position()?;
        if offset.is_zero() {
            return Some(());
        }

        // How much podcast we have
        let duration = self.player.duration()?;
        let destination = match direction {
            // If we are more than `offset` into the podcast, jump back that far
            SeekDirection::Backwards if position >= offset => position.checked_sub(offset),
            // If we haven't played `offset` yet just restart the podcast
            SeekDirection::Backwards if position < offset => Some(ClockTime::from_seconds(0)),
            // If we have more than `offset` remaining jump forward they amount
            SeekDirection::Forward if !duration.is_zero() && position + offset <= duration => {
                position.checked_add(offset)
            }
            // We don't have `offset` remaining just move to the end (ending playback)
            SeekDirection::Forward if !duration.is_zero() && position + offset > duration => {
                Some(duration)
            }
            // Who knows what's going on ¯\_(ツ)_/¯
            _ => None,
        };

        // If we calucated a new position, jump to it
        if let Some(destination) = destination {
            self.player.seek(destination)
        }

        Some(())
    }

    fn rewind(&self) {
        let r = self.seek(ClockTime::from_seconds(10), SeekDirection::Backwards);
        if r.is_none() {
            warn!("Failed to rewind");
        }
    }

    fn fast_forward(&self) {
        let r = self.seek(ClockTime::from_seconds(10), SeekDirection::Forward);
        if r.is_none() {
            warn!("Failed to fast-forward");
        }
    }

    fn set_playback_rate(&self, rate: f64) {
        self.player.set_rate(rate);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerWrapper(pub Rc<RefCell<PlayerWidget>>);

impl Default for PlayerWrapper {
    fn default() -> Self {
        PlayerWrapper(Rc::new(RefCell::new(PlayerWidget::default())))
    }
}

impl Deref for PlayerWrapper {
    type Target = Rc<RefCell<PlayerWidget>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PlayerWrapper {
    pub(crate) fn borrow_mut(&self) -> RefMut<'_, PlayerWidget> {
        self.0.borrow_mut()
    }
    pub(crate) fn new(sender: &Sender<Action>) -> Self {
        let w = PlayerWrapper::default();
        w.init(sender);
        w
    }

    fn init(&self, sender: &Sender<Action>) {
        self.borrow_mut().sender = Some(sender.clone());
        self.connect_control_buttons();
        self.connect_rate_buttons();
        self.connect_mpris_buttons(sender);
        self.connect_gst_signals(sender);
        self.connect_sheet();
    }

    fn connect_sheet(&self) {
        let widget = self.borrow();

        widget
            .timer
            .duration
            .bind_property("label", &widget.sheet.duration, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        widget
            .timer
            .progressed
            .bind_property("label", &widget.sheet.progressed, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        widget
            .sheet
            .slider
            .set_adjustment(&widget.timer.slider.adjustment());
    }

    /// Connect the `PlayerControls` buttons to the `PlayerExt` methods.
    fn connect_control_buttons(&self) {
        let this = self.deref();
        let widget = self.borrow();
        // Connect the play button to the gst Player.
        widget.controls.play.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow().play();
                this.borrow().controls.pause.grab_focus(); // keep focus for accessibility
            }
        ));

        // Connect the pause button to the gst Player.
        widget.controls.pause.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow_mut().pause();
                this.borrow().controls.play.grab_focus(); // keep focus for accessibility
            }
        ));

        // Connect the play button to the gst Player.
        widget.controls.play_small.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow().play();
                this.borrow().controls.pause_small.grab_focus(); // keep focus for accessibility
            }
        ));

        // Connect the pause button to the gst Player.
        widget.controls.pause_small.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow_mut().pause();
                this.borrow().controls.play_small.grab_focus(); // keep focus for accessibility
            }
        ));

        // Connect the rewind button to the gst Player.
        widget.controls.rewind.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow().rewind();
            }
        ));

        // Connect the fast-forward button to the gst Player.
        widget.controls.forward.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow().fast_forward();
            }
        ));

        // Connect the play button to the gst Player.
        widget.sheet.play.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow().play();
                this.borrow().sheet.pause.grab_focus(); // keep focus for accessibility
            }
        ));

        // Connect the pause button to the gst Player.
        widget.sheet.pause.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow_mut().pause();
                this.borrow().sheet.play.grab_focus(); // keep focus for accessibility
            }
        ));

        // Connect the rewind button to the gst Player.
        widget.sheet.rewind.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow().rewind();
            }
        ));

        // Connect the fast-forward button to the gst Player.
        widget.sheet.forward.connect_clicked(clone!(
            #[weak]
            this,
            move |_| {
                this.borrow().fast_forward();
            }
        ));
    }

    fn connect_gst_signals(&self, sender: &Sender<Action>) {
        let signal_adapter = &self.borrow().player_signals;

        // Log gst warnings.
        signal_adapter
            .connect_warning(move |_, warn, details| warn!("gst warning: {} {:#?}", warn, details));

        // Log gst errors.
        signal_adapter.connect_error(clone!(
            #[strong]
            sender,
            move |_, _error, details| {
                error!("gstreamer error: {} {:#?}", _error, details);
                send_blocking!(
                    sender,
                    Action::ErrorNotification(format!("Player Error: {}", _error))
                );
                let s = i18n("The media player was unable to execute an action.");
                send_blocking!(sender, Action::ErrorNotification(s));
            }
        ));

        // The following callbacks require `Send` but are handled by the gtk main loop
        let weak = Fragile::new(Rc::downgrade(self));

        signal_adapter.connect_uri_loaded(clone!(
            #[strong]
            weak,
            move |_, _| {
                if let Some(player_widget) = weak.get().upgrade() {
                    player_widget.borrow().restore_play_position();
                    player_widget.borrow_mut().info.finished_restore = true;
                }
            }
        ));

        // Update the duration label and the slider
        signal_adapter.connect_duration_changed(clone!(
            #[strong]
            weak,
            move |_, clock| {
                if let Some(player_widget) = weak.get().upgrade() {
                    if let Some(c) = clock {
                        player_widget
                            .borrow()
                            .timer
                            .on_duration_changed(Duration(c));
                    }
                }
            }
        ));

        // Update the position label and the slider
        signal_adapter.connect_position_updated(clone!(
            #[strong]
            weak,
            move |_, clock| {
                if let Some(player_widget) = weak.get().upgrade() {
                    // write to db
                    if let Some(c) = clock {
                        let pos = Position(c);
                        let finished_restore = player_widget.borrow().info.finished_restore;
                        player_widget.borrow_mut().info.ep.as_mut().map(|ep| {
                            if finished_restore {
                                ep.set_play_position_if_divergent(pos.seconds() as i32)
                            } else {
                                Ok(())
                            }
                        });
                        player_widget.borrow().timer.on_position_updated(pos);
                        player_widget.borrow().info.update_mpris_position(pos);
                    }
                }
            }
        ));

        // Reset the slider to 0 and show a play button
        signal_adapter.connect_end_of_stream(clone!(
            #[strong]
            sender,
            #[strong]
            weak,
            move |_| {
                if let Some(player_widget) = weak.get().upgrade() {
                    // write postion to db
                    player_widget.borrow_mut().info.ep.as_mut().map(|ep| {
                        ep.set_play_position(0)?;
                        send_blocking!(sender, Action::MarkAsPlayed(true, ep.id()));
                        let ok: Result<(), podcasts_data::errors::DataError> = Ok(());
                        ok
                    });

                    player_widget.borrow_mut().stop()
                }
            }
        ));
    }

    fn connect_rate_buttons(&self) {
        self.deref().borrow().rate.connect_signals(self.deref());
        self.deref()
            .borrow()
            .sheet
            .rate
            .connect_signals(self.deref());
    }

    fn connect_mpris_buttons(&self, sender: &Sender<Action>) {
        let widget = self.borrow();

        if let Some(mpris) = widget.info.mpris.as_ref() {
            mpris.connect_play_pause(clone!(
                #[strong(rename_to = player)]
                self,
                move |mpris| {
                    match mpris.playback_status() {
                        PlaybackStatus::Paused => player.borrow().play(),
                        PlaybackStatus::Stopped => player.borrow().play(),
                        _ => player.borrow_mut().pause(),
                    };
                }
            ));
            mpris.connect_play(clone!(
                #[strong(rename_to = player)]
                self,
                move |_| {
                    player.borrow().play();
                }
            ));

            mpris.connect_pause(clone!(
                #[strong(rename_to = player)]
                self,
                move |_| {
                    player.borrow_mut().pause();
                }
            ));

            mpris.connect_seek(clone!(
                #[strong(rename_to = player)]
                self,
                move |_, offset: mpris_server::Time| {
                    let direction = if offset.is_positive() {
                        SeekDirection::Forward
                    } else {
                        SeekDirection::Backwards
                    };
                    player.borrow().seek(
                        ClockTime::from_useconds(offset.as_micros().unsigned_abs()),
                        direction,
                    );
                }
            ));

            mpris.connect_raise(clone!(
                #[strong]
                sender,
                move |_| {
                    send_blocking!(sender, Action::RaiseWindow);
                }
            ));
        };
    }
}

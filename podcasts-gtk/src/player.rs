// player.rs
//
// Copyright 2018 Jordan Petridis <jpetridis@gnome.org>
// Copyright 2021-2026 nee <nee-git@patchouli.garden>
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
use adw::subclass::prelude::*;
use anyhow::Result;
use async_channel::Sender;
use chrono::prelude::*;
use fragile::Fragile;
use gettextrs::gettext;
use gio::File;
use glib::clone;
use glib::subclass::Signal;
use gst::ClockTime;
use gtk::{gio, glib};
use mpris_server::{self, PlaybackStatus};
use std::cell::{Cell, Ref, RefCell};
use std::ops::Deref;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use crate::app::Action;
use crate::chapter_parser::Chapter;
use crate::player_mpris::PlayerMpris;
use podcasts_data::{
    Episode, EpisodeId, EpisodeModel, ShowCoverModel, ShowId, USER_AGENT, dbqueries,
};

const RATE_MIN: f64 = 0.75;
const RATE_MAX: f64 = 2.0;

/// A Gui independent player.
/// Connect Gui objects to it's signals.
/// Handles podcast playback related features.
/// - getting paths from the DB
/// - playing/streaming with gst
/// - mpris desktop integration
/// - smart rewind
/// - chapters
/// - nextcloud sync
/// - etc...
#[derive(Debug)]
pub struct PlayerPriv {
    player: gst_play::Play,
    // reference needs to be kept, or signals stop firing
    player_signals: gst_play::PlaySignalAdapter,
    mpris: PlayerMpris,
    // currently played episode
    ep: RefCell<Option<Episode>>,
    show: RefCell<Option<ShowCoverModel>>,
    // position restoration after the file loaded into gst
    restore_position: RefCell<Option<i32>>,
    finished_restore: Cell<bool>,

    chapters: RefCell<Vec<Chapter>>,
    // for smart rewind
    last_pause: RefCell<Option<DateTime<Local>>>,
    status: Cell<PlaybackStatus>,
    playback_rate: Cell<f64>,

    sender: RefCell<Option<Sender<Action>>>,
}

impl Default for PlayerPriv {
    fn default() -> Self {
        let player = gst_play::Play::default();
        let mut config = player.config();
        config.set_user_agent(USER_AGENT);
        config.set_position_update_interval(250);
        player.set_config(config).unwrap();
        // A few podcasts have a video track of the thumbnail, which GStreamer displays in a new
        // window. Make sure it doesn't do that.
        player.set_video_track_enabled(false);

        let mpris = PlayerMpris::default();

        let player_signals = gst_play::PlaySignalAdapter::new(&player);
        PlayerPriv {
            player,
            player_signals,
            mpris,
            // currently played episode
            ep: RefCell::new(None),
            show: RefCell::new(None),
            // position restoration after the file loaded into gst
            restore_position: RefCell::new(None),
            finished_restore: Cell::default(),
            chapters: RefCell::new(Vec::new()),
            // for smart rewind
            last_pause: RefCell::new(None),
            sender: RefCell::new(None),
            status: Cell::new(PlaybackStatus::Stopped),
            playback_rate: Cell::new(1.0),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for PlayerPriv {
    const NAME: &'static str = "PdPlayer";
    type ParentType = glib::Object;
    type Type = Player;
}

impl ObjectImpl for PlayerPriv {
    fn signals() -> &'static [Signal] {
        static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| {
            vec![
                Signal::builder("cover-changed").build(),
                Signal::builder("cover-reset").build(),
                Signal::builder("show-changed").build(),
                Signal::builder("episode-changed").build(),
                Signal::builder("status-changed").build(),
                Signal::builder("position-changed")
                    .param_types([glib::Type::U64])
                    .build(),
                Signal::builder("duration-changed")
                    .param_types([glib::Type::U64])
                    .build(),
                Signal::builder("chapters-changed").build(),
                Signal::builder("rate-changed")
                    .param_types([glib::Type::F64])
                    .build(),
            ]
        });

        SIGNALS.as_ref()
    }
}

glib::wrapper! {
    pub struct Player(ObjectSubclass<PlayerPriv>);
}

pub(crate) trait PlayerExt {
    fn play(&self);
    fn pause(&self);
    fn toggle_pause(&self);
    fn stop(&self);
    fn seek(&self, offset: ClockTime, direction: SeekDirection) -> Option<()>;
    fn set_playback_rate(&self, _: f64);
}

pub(crate) trait PlayerUi {
    fn show_changed(&self, show: &ShowCoverModel);
    fn episode_changed(&self, ep: &Episode);
    fn status_changed(&self, status: PlaybackStatus);
    fn show_cover_changed(&self, show: &ShowCoverModel);
    fn show_cover_reset(&self);
    fn position_changed(&self, pos: Position);
    fn duration_changed(&self, duration: Duration);
    fn chapters_changed(&self, _has_chapters: bool) {}
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SeekDirection {
    Backwards,
    Forward,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Duration(ClockTime);

impl Deref for Duration {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Position(pub ClockTime);

impl Deref for Position {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(PartialEq)]
pub enum StreamMode {
    LocalOnly,
    StreamOnly,
    StreamFallback,
}

impl Default for Player {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Player {
    pub(crate) fn init(&self, sender: &Sender<Action>) {
        self.connect_gst_signals(sender);
        self.imp().mpris.init(self, sender);
        self.imp().sender.replace(Some(sender.clone()));
    }

    pub(crate) fn bind_ui<T: IsA<glib::Object> + PlayerUi>(&self, ui: &T) {
        let this = &self;
        // TODO If anyone knows a better <T> definition that fixes #[weak] clone,
        // go ahead and remove `let weak` here
        let weak = ui.downgrade();
        self.connect_local(
            "show-changed",
            false,
            clone!(
                #[weak]
                this,
                #[strong]
                weak,
                #[upgrade_or_default]
                move |_| {
                    if let Some(show) = this.imp().show.borrow().as_ref()
                        && let Some(ui) = weak.upgrade()
                    {
                        ui.show_changed(show);
                    }
                    None
                }
            ),
        );

        self.connect_local(
            "cover-changed",
            false,
            clone!(
                #[weak]
                this,
                #[strong]
                weak,
                #[upgrade_or_default]
                move |_| {
                    if let Some(ui) = weak.upgrade()
                        && let Some(show) = this.imp().show.borrow().as_ref()
                    {
                        ui.show_cover_changed(show);
                    }
                    None
                }
            ),
        );

        self.connect_local(
            "cover-reset",
            false,
            clone!(
                #[strong]
                weak,
                move |_| {
                    if let Some(ui) = weak.upgrade() {
                        ui.show_cover_reset();
                    }
                    None
                }
            ),
        );

        self.connect_local(
            "episode-changed",
            false,
            clone!(
                #[weak]
                this,
                #[strong]
                weak,
                #[upgrade_or_default]
                move |_| {
                    if let Some(ui) = weak.upgrade()
                        && let Some(ep) = this.imp().ep.borrow().as_ref()
                    {
                        ui.episode_changed(ep);
                    }
                    None
                }
            ),
        );

        self.connect_local(
            "status-changed",
            false,
            clone!(
                #[weak]
                this,
                #[strong]
                weak,
                #[upgrade_or_default]
                move |_| {
                    if let Some(ui) = weak.upgrade() {
                        ui.status_changed(this.status());
                    }
                    None
                }
            ),
        );

        self.connect_local(
            "position-changed",
            false,
            clone!(
                #[strong]
                weak,
                move |value| {
                    if let Some(ui) = weak.upgrade() {
                        let seconds: u64 = value[1].get().unwrap_or_default();
                        let position = Position(ClockTime::from_seconds(seconds));
                        ui.position_changed(position);
                    }
                    None
                }
            ),
        );

        self.connect_local(
            "duration-changed",
            false,
            clone!(
                #[strong]
                weak,
                move |value| {
                    if let Some(ui) = weak.upgrade() {
                        let seconds: u64 = value[1].get().unwrap_or_default();
                        let duration = Duration(ClockTime::from_seconds(seconds));
                        ui.duration_changed(duration);
                    }
                    None
                }
            ),
        );

        self.connect_local(
            "chapters-changed",
            false,
            clone!(
                #[weak]
                this,
                #[strong]
                weak,
                #[upgrade_or_default]
                move |_| {
                    if let Some(ui) = weak.upgrade() {
                        ui.chapters_changed(!this.imp().chapters.borrow().is_empty());
                    }
                    None
                }
            ),
        );
    }

    fn connect_gst_signals(&self, sender: &Sender<Action>) {
        let player_signals = &self.imp().player_signals;
        // Log gst warnings.
        player_signals
            .connect_warning(move |_, warn, details| warn!("gst warning: {} {:#?}", warn, details));

        // Log gst errors.
        player_signals.connect_error(clone!(
            #[strong]
            sender,
            move |_, _error, details| {
                error!("gstreamer error: {} {:#?}", _error, details);
                send_blocking!(
                    sender,
                    Action::ErrorNotification(format!("Player Error: {}", _error))
                );
                let s = gettext("The media player was unable to execute an action.");
                send_blocking!(sender, Action::ErrorNotification(s));
            }
        ));

        // The following callbacks require `Send` but are handled by the gtk main loop
        let weak = Fragile::new(self.downgrade());

        player_signals.connect_uri_loaded(clone!(
            #[strong]
            weak,
            move |_, _| {
                if let Some(this) = weak.get().upgrade() {
                    this.restore_play_position();
                    this.imp().finished_restore.set(true);
                }
            }
        ));

        // Update the duration label and the slider
        player_signals.connect_duration_changed(clone!(
            #[strong]
            weak,
            move |_, clock| {
                if let Some(this) = weak.get().upgrade()
                    && let Some(c) = clock
                {
                    this.emit_by_name::<()>("duration-changed", &[&glib::Value::from(c.seconds())]);
                }
            }
        ));

        // Update the position for sliders/labels and store progress in db
        player_signals.connect_position_updated(clone!(
            #[strong]
            weak,
            move |_, clock| {
                if let Some(this) = weak.get().upgrade() {
                    // write to db
                    if let Some(c) = clock {
                        let pos = Position(c);
                        this.imp().ep.borrow_mut().as_mut().map(|ep| {
                            if this.imp().finished_restore.get() {
                                ep.set_play_position_if_divergent(pos.seconds() as i32)
                            } else {
                                Ok(())
                            }
                        });
                        this.on_position_updated(pos);
                    }
                }
            }
        ));

        // Reset the slider to 0 and show a play button
        player_signals.connect_end_of_stream(clone!(
            #[strong]
            sender,
            #[strong]
            weak,
            move |_| {
                if let Some(this) = weak.get().upgrade() {
                    // write postion to db
                    this.imp().ep.borrow_mut().as_mut().map(|ep| {
                        ep.set_play_position_and_save(0)?;

                        if let Err(e) = podcasts_data::sync::Episode::store(
                            ep.id(),
                            podcasts_data::sync::EpisodeAction::Finished,
                            None,
                        ) {
                            error!("Failed to sync {e}");
                        }

                        send_blocking!(sender, Action::MarkAsPlayed(true, ep.id()));
                        let ok: Result<(), podcasts_data::errors::DataError> = Ok(());
                        ok
                    });

                    this.stop()
                }
            }
        ));
    }

    // FIXME: create a Diesel Model of the joined episode and podcast query instead
    fn set_episode_data(
        &self,
        sender: &Sender<Action>,
        episode: &Episode,
        podcast: &ShowCoverModel,
    ) {
        self.imp().ep.replace(Some(episode.clone()));
        self.imp().show.replace(Some(podcast.clone()));
        self.imp().chapters.replace(Vec::new());
        self.emit_by_name::<()>("episode-changed", &[]);
        self.emit_by_name::<()>("show-changed", &[]);
        self.emit_by_name::<()>("chapters-changed", &[]);
        // cover
        let art_path = crate::download_covers::determin_cover_path(podcast, None);
        if art_path.exists() {
            self.emit_by_name::<()>("cover-changed", &[]);
        } else {
            // If the cover art doesn't already exist, download it
            let sender = sender.clone();
            let podcast = podcast.clone();
            self.emit_by_name::<()>("cover-reset", &[]);
            crate::RUNTIME.spawn(async move {
                let id = podcast.id();
                if let Err(err) = crate::download_covers::just_download(&podcast).await {
                    error!("Cover download failed {err}");
                    send!(sender, Action::UpdateCover(id));
                } else {
                    send!(sender, Action::UpdateCover(id));
                }
            });
        }
    }

    pub(crate) fn initialize_episode(
        &self,
        sender: &Sender<Action>,
        id: EpisodeId,
        stream: StreamMode,
        second: Option<i32>,
    ) -> Result<()> {
        let ep = dbqueries::get_episode_from_id(id)?;
        let pd = dbqueries::get_podcast_cover_from_id(ep.show_id())?;

        self.imp().restore_position.replace(second.or_else(|| {
            let episode_position = ep.play_position();
            if episode_position == 0 {
                None
            } else {
                Some(episode_position)
            }
        }));
        let last_id = self.episode_id();
        let is_different_ep = last_id != Some(id);
        self.imp().finished_restore.set(false);
        if is_different_ep {
            self.set_episode_data(sender, &ep, &pd);
            if let Some(last_id) = last_id {
                // refresh the last episode, so it can remove it's pause button
                send_blocking!(sender, Action::RefreshEpisode(last_id));
            }

            // When changing episode, gstreamer resets the playback rate to 1.0 asynchronously,
            // so we need to have a short delay
            // and then set it to the correct rate to avoid race conditions
            if self.imp().playback_rate.get() != 1.0 {
                crate::MAINCONTEXT.spawn_local_with_priority(
                    glib::source::Priority::LOW,
                    clone!(
                        #[strong(rename_to = this)]
                        self,
                        async move {
                            glib::timeout_future(std::time::Duration::from_millis(100)).await;
                            this.set_playback_rate(this.imp().playback_rate.get());
                        }
                    ),
                );
            }
        }

        if stream == StreamMode::StreamOnly {
            if let Some(uri) = ep.uri() {
                self.init_uri(sender, id, uri, second);
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
                self.init_uri(sender, id, uri.as_str(), second);
                return Ok(());
            } else {
                error!("failed to create path for episode {:#?}", ep);
            }
        } else if stream == StreamMode::StreamFallback {
            if let Some(uri) = ep.uri() {
                self.init_uri(sender, id, uri, second);
                return Ok(());
            } else {
                error!("No uri for episode");
            }
        } else {
            error!("Episode not downloaded yet.");
        }

        Ok(())
    }

    fn init_uri(&self, sender: &Sender<Action>, id: EpisodeId, uri: &str, second: Option<i32>) {
        // If it's not the same file load the uri, otherwise just unpause
        let current_uri = self.imp().player.uri();
        if current_uri.as_ref().is_none_or(|s| s != uri) {
            if current_uri.is_some() {
                self.store_position_and_sync();
            }
            self.imp().player.set_uri(Some(uri));

            // fetch chapters
            let uri_string = uri.to_owned();
            crate::RUNTIME.spawn_blocking(clone!(
                #[strong]
                sender,
                move || match crate::chapter_parser::load_chapters(&uri_string) {
                    Ok(chapters) => {
                        send_blocking!(sender, Action::ChaptersAvailable(id, chapters));
                    }
                    Err(e) => {
                        error!("Failed to get chapters: {e:#?}");
                    }
                }
            ));
        } else if second.is_some() {
            // force a jump now if already playing and a jump is given
            self.restore_play_position();
        } else {
            // just unpause, no restore required
            self.imp().finished_restore.set(true);
        }
        // play the file
        self.play();
    }

    // hook for when the async download finished
    pub fn update_cover(&self, show_id: ShowId) -> Result<()> {
        if let Some(ep) = self.imp().ep.borrow().as_ref()
            && ep.show_id() != show_id
        {
            // Download took too long, we are no longer on the same show.
            return Ok(());
        }

        let pd = dbqueries::get_podcast_cover_from_id(show_id)?;
        self.imp().show.replace(Some(pd.clone()));
        self.emit_by_name::<()>("cover-changed", &[]);
        Ok(())
    }

    fn smart_rewind(&self) -> Option<()> {
        static LAST_KNOWN_EPISODE: LazyLock<Mutex<Option<EpisodeId>>> =
            LazyLock::new(|| Mutex::new(None));

        // Figure out the time delta, in seconds, between the last pause and now
        let now = Local::now();
        let last: &Option<DateTime<_>> = &*self.imp().last_pause.borrow();
        let delta = (now - (*last)?).num_seconds();

        // Get interval passed in the gst stream
        let seconds_passed = self.imp().player.position()?.seconds();
        // get the last known episode id
        let mut last = LAST_KNOWN_EPISODE.lock().unwrap();
        // get the current playing episode id
        let current_id = self.episode_id();
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
        debug!("Attempting to restore playback position");
        let pos = *self.imp().restore_position.borrow();
        if let Some(pos) = pos {
            let s: u64 = pos.try_into().ok()?;
            debug!("Seeking stream at second: {}", s);
            self.imp().player.seek(ClockTime::from_seconds(s));
            return Some(());
        }

        debug!("No playback position to restore");
        None
    }

    pub fn chapters_available(&self, id: EpisodeId, chapters: Vec<Chapter>) {
        // cancel if the chapters don't match the currently playing episode anymore.
        if self.episode_id() != Some(id) {
            return;
        }
        self.imp().chapters.replace(chapters);
        self.emit_by_name::<()>("chapters-changed", &[]);
    }

    pub fn change_playback_rate(&self, difference: f64) {
        let rate = (self.imp().player.rate() + difference).clamp(RATE_MIN, RATE_MAX);
        self.set_playback_rate(rate);
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.status() == PlaybackStatus::Playing
    }

    pub(crate) fn status(&self) -> PlaybackStatus {
        self.imp().status.get()
    }

    pub(crate) fn episode(&self) -> Ref<'_, Option<Episode>> {
        self.imp().ep.borrow()
    }

    pub(crate) fn show(&self) -> Ref<'_, Option<ShowCoverModel>> {
        self.imp().show.borrow()
    }

    pub(crate) fn episode_id(&self) -> Option<EpisodeId> {
        self.imp().ep.borrow().as_ref().map(|e| e.id())
    }

    pub(crate) fn chapters(&self) -> Vec<Chapter> {
        self.imp().chapters.borrow().clone()
    }

    pub(crate) fn position(&self) -> Option<Position> {
        let clock = self.imp().player.position();
        clock.map(Position)
    }

    pub(crate) fn on_position_updated(&self, position: Position) {
        self.emit_by_name::<()>(
            "position-changed",
            &[&glib::Value::from(position.seconds())],
        );
    }

    fn store_position_and_sync(&self) {
        let pos = self.imp().player.position();
        if let Some(ep) = self.imp().ep.borrow_mut().as_mut() {
            let start_second = self.imp().restore_position.borrow().unwrap_or(0);
            let second = pos.and_then(|s| s.seconds().try_into().ok()).unwrap_or(0);
            if let Err(e) = podcasts_data::sync::Episode::store(
                ep.id(),
                podcasts_data::sync::EpisodeAction::Play,
                Some((start_second, second)),
            ) {
                error!("Failed to sync {e}");
            }
            if let Err(e) = ep.set_play_position_and_save(second) {
                error!("failed to save episode position {e}");
            }
            if let Some(sender) = self.sender() {
                send_blocking!(sender, Action::QuickSyncNextcloud);
            }
        };
    }

    fn sender(&self) -> Option<Sender<Action>> {
        self.imp().sender.borrow().clone()
    }

    pub(crate) fn jump_to(&self, position: Position) {
        self.imp().player.seek(position.0);
        self.on_position_updated(position);
    }
}

impl PlayerExt for Player {
    fn play(&self) {
        self.smart_rewind();
        self.imp().player.play();
        if let Some(sender) = self.sender() {
            send_blocking!(sender, Action::InhibitSuspend);
            if let Some(id) = self.episode_id() {
                send_blocking!(sender, Action::RefreshEpisode(id));
            }
        }
        self.imp().status.set(PlaybackStatus::Playing);
        self.emit_by_name::<()>("status-changed", &[]);
    }

    fn pause(&self) {
        self.imp().player.pause();
        self.imp().last_pause.replace(Some(Local::now()));

        self.store_position_and_sync();
        self.imp().status.set(PlaybackStatus::Paused);
        self.emit_by_name::<()>("status-changed", &[]);
        if let Some(sender) = self.sender() {
            send_blocking!(sender, Action::UninhibitSuspend);
            if let Some(id) = self.episode_id() {
                send_blocking!(sender, Action::RefreshEpisode(id));
            }
        }
    }

    fn stop(&self) {
        self.imp().ep.replace(None);
        self.imp().restore_position.replace(None);
        self.imp().player.stop();

        // Reset the slider bar to the start
        self.on_position_updated(Position(ClockTime::from_seconds(0)));
        self.imp().status.set(PlaybackStatus::Stopped);
        self.emit_by_name::<()>("status-changed", &[]);

        if let Some(sender) = self.sender() {
            send_blocking!(sender, Action::UninhibitSuspend);
            if let Some(id) = self.episode_id() {
                send_blocking!(sender, Action::RefreshEpisode(id));
            }
        }
    }

    fn toggle_pause(&self) {
        match self.status() {
            PlaybackStatus::Paused => self.play(),
            PlaybackStatus::Stopped => self.play(),
            _ => self.pause(),
        };
    }

    // Adapted from https://github.com/philn/glide/blob/b52a65d99daeab0b487f79a0e1ccfad0cd433e22/src/player_context.rs#L219-L245
    fn seek(&self, offset: ClockTime, direction: SeekDirection) -> Option<()> {
        // How far into the podcast we are
        let position = self.imp().player.position()?;
        if offset.is_zero() {
            return Some(());
        }

        // How much podcast we have
        let duration = self.imp().player.duration()?;
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
        }?;

        self.imp().player.seek(destination);
        self.on_position_updated(Position(destination));
        Some(())
    }

    fn set_playback_rate(&self, rate: f64) {
        self.imp().player.set_rate(rate);
        self.imp().playback_rate.set(rate);
        self.emit_by_name::<()>("rate-changed", &[&glib::Value::from(rate)]);
    }
}

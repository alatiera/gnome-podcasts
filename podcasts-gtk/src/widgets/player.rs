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

use gst::prelude::*;
use gst::ClockTime;
use gst_player;

use gtk;
use gtk::prelude::*;

use gio::{File, FileExt};
use glib::{SignalHandlerId, WeakRef};

use chrono::{prelude::*, NaiveTime};
use crossbeam_channel::Sender;
use failure::Error;
use fragile::Fragile;

use podcasts_data::{dbqueries, USER_AGENT};
use podcasts_data::{EpisodeWidgetModel, ShowCoverModel};

use app::Action;
use utils::set_image_from_path;

use std::cell::RefCell;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

use i18n::i18n;

use mpris_player::{Metadata, MprisPlayer, OrgMprisMediaPlayer2Player, PlaybackStatus};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
enum SeekDirection {
    Backwards,
    Forward,
}

trait PlayerExt {
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
    fn seek(&self, offset: ClockTime, direction: SeekDirection);
    fn fast_forward(&self);
    fn rewind(&self);
    fn set_playback_rate(&self, f64);
}

#[derive(Debug, Clone)]
struct PlayerInfo {
    container: gtk::Box,
    show: gtk::Label,
    episode: gtk::Label,
    cover: gtk::Image,
    mpris: Arc<MprisPlayer>,
    episode_id: RefCell<Option<i32>>,
}

impl PlayerInfo {
    // FIXME: create a Diesel Model of the joined episode and podcast query instead
    fn init(&self, episode: &EpisodeWidgetModel, podcast: &ShowCoverModel) {
        self.episode_id.replace(Some(episode.rowid()));
        self.set_cover_image(podcast);
        self.set_show_title(podcast);
        self.set_episode_title(episode);

        let mut metadata = Metadata::new();
        metadata.artist = Some(vec![podcast.title().to_string()]);
        metadata.title = Some(episode.title().to_string());
        // FIXME: .image_uri() returns an http url, we should instead
        // pass it the local path to the downloaded cover image.
        metadata.art_url = podcast.image_uri().clone().map(From::from);

        self.mpris.set_metadata(metadata);
        self.mpris.set_can_play(true);
    }

    fn set_episode_title(&self, episode: &EpisodeWidgetModel) {
        self.episode.set_text(episode.title());
        self.episode.set_tooltip_text(episode.title());
    }

    fn set_show_title(&self, show: &ShowCoverModel) {
        self.show.set_text(show.title());
        self.show.set_tooltip_text(show.title());
    }

    fn set_cover_image(&self, show: &ShowCoverModel) {
        set_image_from_path(&self.cover, show.id(), 34)
            .map_err(|err| error!("Player Cover: {}", err))
            .ok();
    }
}

#[derive(Debug, Clone)]
struct PlayerTimes {
    container: gtk::Box,
    progressed: gtk::Label,
    duration: gtk::Label,
    separator: gtk::Label,
    slider: gtk::Scale,
    slider_update: Rc<SignalHandlerId>,
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
        let seconds = duration.seconds().map(|v| v as f64).unwrap_or(0.0);

        self.slider.block_signal(&self.slider_update);
        self.slider.set_range(0.0, seconds);
        self.slider.unblock_signal(&self.slider_update);

        self.duration.set_text(&format_duration(seconds as u32));
    }

    /// Update the `gtk::Scale` bar when the pipeline position is changed.
    pub(crate) fn on_position_updated(&self, position: Position) {
        let seconds = position.seconds().map(|v| v as f64).unwrap_or(0.0);

        self.slider.block_signal(&self.slider_update);
        self.slider.set_value(seconds);
        self.slider.unblock_signal(&self.slider_update);

        self.progressed.set_text(&format_duration(seconds as u32));
    }
}

fn format_duration(seconds: u32) -> String {
    let time = NaiveTime::from_num_seconds_from_midnight(seconds, 0);

    if seconds >= 3600 {
        time.format("%T").to_string()
    } else {
        time.format("%M∶%S").to_string()
    }
}

#[derive(Debug, Clone)]
struct PlayerRate {
    radio150: gtk::RadioButton,
    radio125: gtk::RadioButton,
    radio_normal: gtk::RadioButton,
    popover: gtk::Popover,
    btn: gtk::MenuButton,
    label: gtk::Label,
}

#[derive(Debug, Clone)]
struct PlayerControls {
    container: gtk::Box,
    play: gtk::Button,
    pause: gtk::Button,
    forward: gtk::Button,
    rewind: gtk::Button,
    last_pause: RefCell<Option<DateTime<Local>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerWidget {
    pub(crate) action_bar: gtk::ActionBar,
    player: gst_player::Player,
    controls: PlayerControls,
    timer: PlayerTimes,
    info: PlayerInfo,
    rate: PlayerRate,
}

impl Default for PlayerWidget {
    fn default() -> Self {
        let dispatcher = gst_player::PlayerGMainContextSignalDispatcher::new(None);
        let player = gst_player::Player::new(
            None,
            // Use the gtk main thread
            Some(&dispatcher.upcast::<gst_player::PlayerSignalDispatcher>()),
        );

        let mpris = MprisPlayer::new(
            "Podcasts".to_string(),
            "GNOME Podcasts".to_string(),
            "org.gnome.Podcasts.desktop".to_string(),
        );
        mpris.set_can_raise(true);
        mpris.set_can_play(false);
        mpris.set_can_seek(false);
        mpris.set_can_set_fullscreen(false);

        let mut config = player.get_config();
        config.set_user_agent(USER_AGENT);
        config.set_position_update_interval(250);
        player.set_config(config).unwrap();

        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/player_toolbar.ui");
        let action_bar = builder.get_object("action_bar").unwrap();

        let buttons = builder.get_object("buttons").unwrap();
        let play = builder.get_object("play_button").unwrap();
        let pause = builder.get_object("pause_button").unwrap();
        let forward: gtk::Button = builder.get_object("ff_button").unwrap();
        let rewind: gtk::Button = builder.get_object("rewind_button").unwrap();

        let controls = PlayerControls {
            container: buttons,
            play,
            pause,
            forward,
            rewind,
            last_pause: RefCell::new(None),
        };

        let timer_container = builder.get_object("timer").unwrap();
        let progressed = builder.get_object("progress_time_label").unwrap();
        let duration = builder.get_object("total_duration_label").unwrap();
        let separator = builder.get_object("separator").unwrap();
        let slider: gtk::Scale = builder.get_object("seek").unwrap();
        slider.set_range(0.0, 1.0);
        let player_weak = player.downgrade();
        let slider_update = Rc::new(Self::connect_update_slider(&slider, player_weak));
        let timer = PlayerTimes {
            container: timer_container,
            progressed,
            duration,
            separator,
            slider,
            slider_update,
        };

        let labels = builder.get_object("info").unwrap();
        let show = builder.get_object("show_label").unwrap();
        let episode = builder.get_object("episode_label").unwrap();
        let cover = builder.get_object("show_cover").unwrap();
        let info = PlayerInfo {
            mpris,
            container: labels,
            show,
            episode,
            cover,
            episode_id: RefCell::new(None),
        };

        let radio150 = builder.get_object("rate_1_50").unwrap();
        let radio125 = builder.get_object("rate_1_25").unwrap();
        let radio_normal = builder.get_object("normal_rate").unwrap();
        let popover = builder.get_object("rate_popover").unwrap();
        let btn = builder.get_object("rate_button").unwrap();
        let label = builder.get_object("rate_label").unwrap();
        let rate = PlayerRate {
            radio150,
            radio125,
            radio_normal,
            popover,
            label,
            btn,
        };

        PlayerWidget {
            player,
            action_bar,
            controls,
            timer,
            info,
            rate,
        }
    }
}

impl PlayerWidget {
    pub(crate) fn new(sender: &Sender<Action>) -> Rc<Self> {
        let w = Rc::new(Self::default());
        Self::init(&w, sender);
        w
    }

    fn init(s: &Rc<Self>, sender: &Sender<Action>) {
        Self::connect_control_buttons(s);
        Self::connect_rate_buttons(s);
        Self::connect_mpris_buttons(s, sender);
        Self::connect_gst_signals(s, sender);
    }

    /// Connect the `PlayerControls` buttons to the `PlayerExt` methods.
    fn connect_control_buttons(s: &Rc<Self>) {
        let weak = Rc::downgrade(s);

        // Connect the play button to the gst Player.
        s.controls.play.connect_clicked(clone!(weak => move |_| {
             weak.upgrade().map(|p| p.play());
        }));

        // Connect the pause button to the gst Player.
        s.controls.pause.connect_clicked(clone!(weak => move |_| {
            weak.upgrade().map(|p| p.pause());
        }));

        // Connect the rewind button to the gst Player.
        s.controls.rewind.connect_clicked(clone!(weak => move |_| {
            weak.upgrade().map(|p| p.rewind());
        }));

        // Connect the fast-forward button to the gst Player.
        s.controls.forward.connect_clicked(clone!(weak => move |_| {
            weak.upgrade().map(|p| p.fast_forward());
        }));
    }

    fn connect_mpris_buttons(s: &Rc<Self>, sender: &Sender<Action>) {
        let weak = Rc::downgrade(s);

        let mpris = s.info.mpris.clone();
        s.info.mpris.connect_play_pause(clone!(weak => move || {
            let player = match weak.upgrade() {
                Some(s) => s,
                None => return
            };

            if let Ok(status) = mpris.get_playback_status() {
                match status.as_ref() {
                    "Paused" => player.play(),
                    "Stopped" => player.play(),
                    _ => player.pause(),
                };
            }
        }));

        s.info.mpris.connect_next(clone!(weak => move || {
            weak.upgrade().map(|p| p.fast_forward());
        }));

        s.info.mpris.connect_previous(clone!(weak => move || {
            weak.upgrade().map(|p| p.rewind());
        }));

        s.info
            .mpris
            .connect_raise(clone!(sender => move || sender.send(Action::RaiseWindow)));
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn connect_gst_signals(s: &Rc<Self>, sender: &Sender<Action>) {
        // Log gst warnings.
        s.player.connect_warning(move |_, warn| warn!("gst warning: {}", warn));

        // Log gst errors.
        s.player.connect_error(clone!(sender => move |_, _error| {
            // sender.send(Action::ErrorNotification(format!("Player Error: {}", error)));
            let s = i18n("The media player was unable to execute an action.");
            sender.send(Action::ErrorNotification(s));
        }));

        // The following callbacks require `Send` but are handled by the gtk main loop
        let weak = Fragile::new(Rc::downgrade(s));

        // Update the duration label and the slider
        s.player.connect_duration_changed(clone!(weak => move |_, clock| {
            weak.get()
                .upgrade()
                .map(|p| p.timer.on_duration_changed(Duration(clock)));
        }));

        // Update the position label and the slider
        s.player.connect_position_updated(clone!(weak => move |_, clock| {
            weak.get()
                .upgrade()
                .map(|p| p.timer.on_position_updated(Position(clock)));
        }));

        // Reset the slider to 0 and show a play button
        s.player.connect_end_of_stream(clone!(weak => move |_| {
             weak.get()
                 .upgrade()
                 .map(|p| p.stop());
        }));
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn connect_rate_buttons(s: &Rc<Self>) {
        let weak = Rc::downgrade(s);

        s.rate
            .radio_normal
            .connect_toggled(clone!(weak => move |_| {
                weak.upgrade().map(|p| p.on_rate_changed(1.00));
            }));

        s.rate
            .radio125
            .connect_toggled(clone!(weak => move |_| {
                weak.upgrade().map(|p| p.on_rate_changed(1.25));
            }));

        s.rate
            .radio150
            .connect_toggled(clone!(weak => move |_| {
                weak.upgrade().map(|p| p.on_rate_changed(1.50));
            }));
    }

    fn on_rate_changed(&self, rate: f64) {
        self.set_playback_rate(rate);
        self.rate.label.set_text(&format!("{:.2}×", rate));
    }

    fn reveal(&self) {
        self.action_bar.show();
    }

    pub(crate) fn initialize_episode(&self, rowid: i32) -> Result<(), Error> {
        let ep = dbqueries::get_episode_widget_from_rowid(rowid)?;
        let pd = dbqueries::get_podcast_cover_from_id(ep.show_id())?;

        self.info.init(&ep, &pd);
        // Currently that will always be the case since the play button is
        // only shown if the file is downloaded
        if let Some(ref path) = ep.local_uri() {
            if Path::new(path).exists() {
                // path is an absolute fs path ex. "foo/bar/baz".
                // Convert it so it will have a "file:///"
                // FIXME: convert it properly
                if let Some(uri) = File::new_for_path(path).get_uri() {
                    // play the file
                    self.player.set_uri(&uri);
                    self.play();
                    return Ok(());
                }
            }
            // TODO: log an error
        }

        // FIXME: Stream stuff
        // unimplemented!()
        Ok(())
    }

    fn connect_update_slider(
        slider: &gtk::Scale,
        player: WeakRef<gst_player::Player>,
    ) -> SignalHandlerId {
        slider.connect_value_changed(move |slider| {
            let player = match player.upgrade() {
                Some(p) => p,
                None => return,
            };

            let value = slider.get_value() as u64;
            player.seek(ClockTime::from_seconds(value));
        })
    }

    fn smart_rewind(&self) -> Option<()> {
        lazy_static! {
            static ref LAST_KNOWN_EPISODE: Mutex<Option<i32>> = Mutex::new(None);
        };

        // Figure out the time delta, in seconds, between the last pause and now
        let now = Local::now();
        let last: &Option<DateTime<_>> = &*self.controls.last_pause.borrow();
        let last = last.clone()?;
        let delta = (now - last).num_seconds();

        // Get interval passed in the gst stream
        let seconds_passed = self.player.get_position().seconds()?;
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
}

impl PlayerExt for PlayerWidget {
    fn play(&self) {
        self.reveal();

        self.controls.pause.show();
        self.controls.play.hide();

        self.smart_rewind();
        self.player.play();
        self.info.mpris.set_playback_status(PlaybackStatus::Playing);
    }

    fn pause(&self) {
        self.controls.pause.hide();
        self.controls.play.show();

        self.player.pause();
        self.info.mpris.set_playback_status(PlaybackStatus::Paused);

        self.controls.last_pause.replace(Some(Local::now()));
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn stop(&self) {
        self.controls.pause.hide();
        self.controls.play.show();

        self.player.stop();
        self.info.mpris.set_playback_status(PlaybackStatus::Paused);

        // Reset the slider bar to the start
        self.timer.on_position_updated(Position(ClockTime::from_seconds(0)));
    }

    // Adapted from https://github.com/philn/glide/blob/b52a65d99daeab0b487f79a0e1ccfad0cd433e22/src/player_context.rs#L219-L245
    fn seek(&self, offset: ClockTime, direction: SeekDirection) {
        // How far into the podcast we are
        let position = self.player.get_position();
        if position.is_none() || offset.is_none() {
            return;
        }

        // How much podcast we have
        let duration = self.player.get_duration();
        let destination = match direction {
            // If we are more than `offset` into the podcast, jump back that far
            SeekDirection::Backwards if position >= offset => Some(position - offset),
            // If we haven't played `offset` yet just restart the podcast
            SeekDirection::Backwards if position < offset => Some(ClockTime::from_seconds(0)),
            // If we have more than `offset` remaining jump forward they amount
            SeekDirection::Forward if !duration.is_none() && position + offset <= duration => {
                Some(position + offset)
            }
            // We don't have `offset` remaining just move to the end (ending playback)
            SeekDirection::Forward if !duration.is_none() && position + offset > duration => {
                Some(duration)
            }
            // Who knows what's going on ¯\_(ツ)_/¯
            _ => None,
        };

        // If we calucated a new position, jump to it
        destination.map(|d| self.player.seek(d));
    }

    fn rewind(&self) {
        self.seek(ClockTime::from_seconds(10), SeekDirection::Backwards)
    }

    fn fast_forward(&self) {
        self.seek(ClockTime::from_seconds(10), SeekDirection::Forward)
    }

    fn set_playback_rate(&self, rate: f64) {
        self.player.set_rate(rate);
    }
}

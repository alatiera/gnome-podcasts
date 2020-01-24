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

use gst::ClockTime;

use gtk;
use gtk::prelude::*;

use libhandy as hdy;
use libhandy::prelude::*;

use gio::{File, FileExt};
use glib::clone;
use glib::{SignalHandlerId, WeakRef};

use chrono::{prelude::*, NaiveTime};
use crossbeam_channel::Sender;
use failure::Error;
use fragile::Fragile;

use podcasts_data::{dbqueries, USER_AGENT};
use podcasts_data::{EpisodeWidgetModel, ShowCoverModel};

use crate::app::Action;
use crate::config::APP_ID;
use crate::utils::set_image_from_path;

use std::cell::RefCell;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

use crate::i18n::i18n;

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
    fn set_playback_rate(&self, _: f64);
}

#[derive(Debug, Clone)]
struct PlayerInfo {
    container: gtk::Box,
    show: gtk::Label,
    episode: gtk::Label,
    cover: gtk::Image,
    show_small: gtk::Label,
    episode_small: gtk::Label,
    cover_small: gtk::Image,
    mpris: Arc<MprisPlayer>,
    episode_id: RefCell<Option<i32>>,
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
        self.cover
            .bind_property("pixbuf", &self.cover_small, "pixbuf")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
    }

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
        self.episode.set_tooltip_text(Some(episode.title()));
    }

    fn set_show_title(&self, show: &ShowCoverModel) {
        self.show.set_text(show.title());
        self.show.set_tooltip_text(Some(show.title()));
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
        let seconds = duration.seconds().map(|v| v as f64).unwrap_or(0.0);

        self.slider.block_signal(&self.slider_update);
        self.slider.set_range(0.0, seconds);
        self.slider.unblock_signal(&self.slider_update);

        self.duration.set_text(&format_duration(seconds as u32));

        self.update_progress_bar();
    }

    /// Update the `gtk::Scale` bar when the pipeline position is changed.
    pub(crate) fn on_position_updated(&self, position: Position) {
        let seconds = position.seconds().map(|v| v as f64).unwrap_or(0.0);

        self.slider.block_signal(&self.slider_update);
        self.slider.set_value(seconds);
        self.slider.unblock_signal(&self.slider_update);

        self.progressed.set_text(&format_duration(seconds as u32));

        self.update_progress_bar();
    }

    fn update_progress_bar(&self) {
        let fraction = self.slider.get_value() / self.slider.get_adjustment().get_upper();
        self.progress_bar.set_fraction(fraction);
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
    radio150: gtk::ModelButton,
    radio125: gtk::ModelButton,
    radio_normal: gtk::ModelButton,
    popover: gtk::Popover,
    btn: gtk::MenuButton,
    label: gtk::Label,
}

impl PlayerRate {
    fn new() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/player_rate.ui");

        let radio150: gtk::ModelButton = builder.get_object("rate_1_50").unwrap();
        let radio125: gtk::ModelButton = builder.get_object("rate_1_25").unwrap();
        let radio_normal: gtk::ModelButton = builder.get_object("normal_rate").unwrap();
        let popover = builder.get_object("rate_popover").unwrap();
        let btn = builder.get_object("rate_button").unwrap();
        let label = builder.get_object("rate_label").unwrap();

        PlayerRate {
            radio150,
            radio125,
            radio_normal,
            popover,
            label,
            btn,
        }
    }

    fn set_rate(&self, rate: f64) {
        self.label.set_text(&format!("{:.2}×", rate));
        self.radio_normal.set_property_active(rate == 1.0);
        self.radio125.set_property_active(rate == 1.25);
        self.radio150.set_property_active(rate == 1.5);
    }

    fn connect_signals(&self, widget: &Rc<PlayerWidget>) {
        self.radio_normal
            .connect_clicked(clone!(@weak widget => move |_| {
                widget.on_rate_changed(1.00);
            }));
        self.radio125
            .connect_clicked(clone!(@weak widget => move |_| {
                widget.on_rate_changed(1.25);
            }));
        self.radio150
            .connect_clicked(clone!(@weak widget => move |_| {
                widget.on_rate_changed(1.50);
            }));
    }
}

#[derive(Debug, Clone)]
struct PlayerControls {
    container: gtk::Box,
    play: gtk::Button,
    pause: gtk::Button,
    play_small: gtk::Button,
    pause_small: gtk::Button,
    play_pause_small: gtk::Stack,
    forward: gtk::Button,
    rewind: gtk::Button,
    last_pause: RefCell<Option<DateTime<Local>>>,
}

#[derive(Debug, Clone)]
struct PlayerDialog {
    dialog: hdy::Dialog,
    close: gtk::Button,
    headerbar: hdy::HeaderBar,
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
}

impl PlayerDialog {
    fn new(rate: PlayerRate) -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/player_dialog.ui");
        let dialog = builder.get_object("dialog").unwrap();

        let close = builder.get_object("close").unwrap();
        let headerbar = builder.get_object("headerbar").unwrap();
        let cover = builder.get_object("cover").unwrap();
        let play_pause = builder.get_object("play_pause").unwrap();
        let play = builder.get_object("play").unwrap();
        let pause = builder.get_object("pause").unwrap();
        let duration = builder.get_object("duration").unwrap();
        let progressed = builder.get_object("progressed").unwrap();
        let slider = builder.get_object("slider").unwrap();
        let rewind = builder.get_object("rewind").unwrap();
        let forward = builder.get_object("forward").unwrap();
        let bottom: gtk::Box = builder.get_object("bottom").unwrap();
        let show = builder.get_object("show_label").unwrap();
        let episode = builder.get_object("episode_label").unwrap();

        bottom.pack_start(&rate.btn, false, true, 0);

        PlayerDialog {
            dialog,
            close,
            headerbar,
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
        }
    }

    fn initialize_episode(&self, episode: &EpisodeWidgetModel, show: &ShowCoverModel) {
        self.episode.set_text(episode.title());
        self.show.set_text(show.title());

        set_image_from_path(&self.cover, show.id(), 256)
            .map_err(|err| error!("Player Cover: {}", err))
            .ok();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlayerWidget {
    pub(crate) container: gtk::Box,
    action_bar: gtk::ActionBar,
    evbox: gtk::EventBox,
    player: gst_player::Player,
    controls: PlayerControls,
    dialog: PlayerDialog,
    full: gtk::Box,
    squeezer: hdy::Squeezer,
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

        // A few podcasts have a video track of the thumbnail, which GStreamer displays in a new
        // window. Make sure it doesn't do that.
        player.set_video_track_enabled(false);

        let mpris = MprisPlayer::new(
            APP_ID.to_string(),
            "GNOME Podcasts".to_string(),
            format!("{}.desktop", APP_ID).to_string(),
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

        let buttons = builder.get_object("buttons").unwrap();
        let play = builder.get_object("play_button").unwrap();
        let pause = builder.get_object("pause_button").unwrap();
        let play_small = builder.get_object("play_button_small").unwrap();
        let pause_small = builder.get_object("pause_button_small").unwrap();
        let forward: gtk::Button = builder.get_object("ff_button").unwrap();
        let rewind: gtk::Button = builder.get_object("rewind_button").unwrap();
        let play_pause_small = builder.get_object("play_pause_small").unwrap();

        let controls = PlayerControls {
            container: buttons,
            play,
            pause,
            play_small,
            pause_small,
            play_pause_small,
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
        let progress_bar = builder.get_object("progress_bar").unwrap();
        let timer = PlayerTimes {
            container: timer_container,
            progressed,
            duration,
            separator,
            slider,
            slider_update,
            progress_bar,
        };

        let labels = builder.get_object("info").unwrap();
        let show = builder.get_object("show_label").unwrap();
        let episode = builder.get_object("episode_label").unwrap();
        let cover = builder.get_object("show_cover").unwrap();
        let show_small = builder.get_object("show_label_small").unwrap();
        let episode_small = builder.get_object("episode_label_small").unwrap();
        let cover_small = builder.get_object("show_cover_small").unwrap();
        let info = PlayerInfo {
            mpris,
            container: labels,
            show,
            episode,
            cover,
            show_small,
            episode_small,
            cover_small,
            episode_id: RefCell::new(None),
        };
        info.create_bindings();

        let dialog_rate = PlayerRate::new();
        let dialog = PlayerDialog::new(dialog_rate);

        let container = builder.get_object("container").unwrap();
        let action_bar: gtk::ActionBar = builder.get_object("action_bar").unwrap();
        let evbox = builder.get_object("evbox").unwrap();
        let full: gtk::Box = builder.get_object("full").unwrap();
        let squeezer = builder.get_object("squeezer").unwrap();

        let rate = PlayerRate::new();
        full.pack_end(&rate.btn, false, true, 0);

        PlayerWidget {
            player,
            container,
            action_bar,
            evbox,
            controls,
            dialog,
            full,
            squeezer,
            timer,
            info,
            rate,
        }
    }
}

impl PlayerWidget {
    fn on_rate_changed(&self, rate: f64) {
        self.set_playback_rate(rate);
        self.rate.set_rate(rate);
        self.dialog.rate.set_rate(rate);
    }

    fn reveal(&self) {
        self.action_bar.show();
    }

    pub(crate) fn initialize_episode(&self, rowid: i32) -> Result<(), Error> {
        let ep = dbqueries::get_episode_widget_from_rowid(rowid)?;
        let pd = dbqueries::get_podcast_cover_from_id(ep.show_id())?;

        self.dialog.initialize_episode(&ep, &pd);

        self.info.init(&ep, &pd);
        // Currently that will always be the case since the play button is
        // only shown if the file is downloaded
        if let Some(ref path) = ep.local_uri() {
            if Path::new(path).exists() {
                // path is an absolute fs path ex. "foo/bar/baz".
                // Convert it so it will have a "file:///"
                // FIXME: convert it properly
                let uri = File::new_for_path(path).get_uri();
                // play the file
                self.player.set_uri(uri.as_str());
                self.rate.set_rate(1.0);
                self.dialog.rate.set_rate(1.0);
                self.play();

                return Ok(());
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
        self.dialog.play_pause.set_visible_child(&self.dialog.pause);

        self.reveal();

        self.controls.pause.show();
        self.controls.play.hide();
        self.controls
            .play_pause_small
            .set_visible_child(&self.controls.pause_small);

        self.smart_rewind();
        self.player.play();
        self.info.mpris.set_playback_status(PlaybackStatus::Playing);
    }

    fn pause(&self) {
        self.dialog.play_pause.set_visible_child(&self.dialog.play);

        self.controls.pause.hide();
        self.controls.play.show();
        self.controls
            .play_pause_small
            .set_visible_child(&self.controls.play_small);

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

#[derive(Debug, Clone)]
pub(crate) struct PlayerWrapper(pub Rc<PlayerWidget>);

impl Default for PlayerWrapper {
    fn default() -> Self {
        PlayerWrapper(Rc::new(PlayerWidget::default()))
    }
}

impl Deref for PlayerWrapper {
    type Target = Rc<PlayerWidget>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PlayerWrapper {
    pub(crate) fn new(sender: &Sender<Action>) -> Self {
        let w = Self::default();
        w.init(sender);
        w
    }

    fn init(&self, sender: &Sender<Action>) {
        self.connect_control_buttons();
        self.connect_rate_buttons();
        self.connect_mpris_buttons(sender);
        self.connect_gst_signals(sender);
        self.connect_dialog();
    }

    fn connect_dialog(&self) {
        let this = self.deref();
        self.squeezer
            .connect_property_visible_child_notify(clone!(@weak this => move |_| {
                    if let Some(child) = this.squeezer.get_visible_child() {
                        let full = child == this.full;
                        this.timer.progress_bar.set_visible(!full);
                        if full {
                            this.action_bar.get_style_context().remove_class("player-small");
                        } else {
                            this.action_bar.get_style_context().add_class("player-small");
                        }
                    }
            }));

        self.timer
            .duration
            .bind_property("label", &self.dialog.duration, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        self.timer
            .progressed
            .bind_property("label", &self.dialog.progressed, "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        self.dialog
            .slider
            .set_adjustment(&self.timer.slider.get_adjustment());

        self.evbox.connect_button_press_event(
            clone!(@weak this =>  @default-return Inhibit(false), move |_, event| {
                if event.get_button() != 1 {
                    return Inhibit(false);
                }
                // only open the dialog when the small toolbar is visible
                if let Some(child) = this.squeezer.get_visible_child() {
                    if child == this.full {
                        return Inhibit(false);
                    }
                }

                let parent = this.container.get_toplevel().and_then(|toplevel| {
                    toplevel
                        .downcast::<gtk::Window>()
                        .ok()
                }).unwrap();

                info!("showing dialog");
                this.dialog.dialog.set_transient_for(Some(&parent));
                this.dialog.dialog.show();

                Inhibit(false)
            }),
        );

        self.dialog
            .close
            .connect_clicked(clone!(@weak this => move |_| {
                    this.dialog.dialog.hide();
            }));
    }

    /// Connect the `PlayerControls` buttons to the `PlayerExt` methods.
    fn connect_control_buttons(&self) {
        let this = self.deref();
        // Connect the play button to the gst Player.
        self.controls
            .play
            .connect_clicked(clone!(@weak this => move |_| {
                     this.play();
            }));

        // Connect the pause button to the gst Player.
        self.controls
            .pause
            .connect_clicked(clone!(@weak this => move |_| {
                this.pause();
            }));

        // Connect the play button to the gst Player.
        self.controls
            .play_small
            .connect_clicked(clone!(@weak this => move |_| {
                 this.play();
            }));

        // Connect the pause button to the gst Player.
        self.controls
            .pause_small
            .connect_clicked(clone!(@weak this => move |_| {
                this.pause();
            }));

        // Connect the rewind button to the gst Player.
        self.controls
            .rewind
            .connect_clicked(clone!(@weak this => move |_| {
                this.rewind();
            }));

        // Connect the fast-forward button to the gst Player.
        self.controls
            .forward
            .connect_clicked(clone!(@weak this => move |_| {
                this.fast_forward();
            }));

        // Connect the play button to the gst Player.
        self.dialog
            .play
            .connect_clicked(clone!(@weak this => move |_| {
                     this.play();
            }));

        // Connect the pause button to the gst Player.
        self.dialog
            .pause
            .connect_clicked(clone!(@weak this => move |_| {
                    this.pause();
            }));

        // Connect the rewind button to the gst Player.
        self.dialog
            .rewind
            .connect_clicked(clone!(@weak this => move |_| {
                    this.rewind();
            }));

        // Connect the fast-forward button to the gst Player.
        self.dialog
            .forward
            .connect_clicked(clone!(@weak this => move |_| {
                this.fast_forward();
            }));
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn connect_gst_signals(&self, sender: &Sender<Action>) {
        // Log gst warnings.
        self.player.connect_warning(move |_, warn| warn!("gst warning: {}", warn));

        // Log gst errors.
        self.player.connect_error(clone!(@strong sender => move |_, _error| {
            // sender.send(Action::ErrorNotification(format!("Player Error: {}", error)));
            let s = i18n("The media player was unable to execute an action.");
            sender.send(Action::ErrorNotification(s)).expect("Action channel blew up somehow");
        }));

        // The following callbacks require `Send` but are handled by the gtk main loop
        let weak = Fragile::new(Rc::downgrade(self));

        // Update the duration label and the slider
        self.player.connect_duration_changed(clone!(@strong weak => move |_, clock| {
            weak.get()
                .upgrade()
                .map(|p| p.timer.on_duration_changed(Duration(clock)));
        }));

        // Update the position label and the slider
        self.player.connect_position_updated(clone!(@strong weak => move |_, clock| {
            weak.get()
                .upgrade()
                .map(|p| p.timer.on_position_updated(Position(clock)));
        }));

        // Reset the slider to 0 and show a play button
        self.player.connect_end_of_stream(clone!(@strong weak => move |_| {
             weak.get()
                 .upgrade()
                 .map(|p| p.stop());
        }));
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn connect_rate_buttons(&self) {
        self.rate.connect_signals(self);
        self.dialog.rate.connect_signals(self);
    }

    fn connect_mpris_buttons(&self, sender: &Sender<Action>) {
        let weak = Rc::downgrade(self);

        // FIXME: Reference cycle with mpris
        let mpris = self.info.mpris.clone();
        self.info
            .mpris
            .connect_play_pause(clone!(@strong weak => move || {
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
        self.info
            .mpris
            .connect_play(clone!(@strong weak => move || {
                let player = match weak.upgrade() {
                    Some(s) => s,
                    None => return
                };

                player.play();
            }));

        self.info
            .mpris
            .connect_pause(clone!(@strong weak => move || {
                let player = match weak.upgrade() {
                    Some(s) => s,
                    None => return
                };

                player.pause();
            }));

        self.info
            .mpris
            .connect_next(clone!(@strong weak => move || {
                weak.upgrade().map(|p| p.fast_forward());
            }));

        self.info
            .mpris
            .connect_previous(clone!(@strong weak => move || {
                weak.upgrade().map(|p| p.rewind());
            }));

        self.info
            .mpris
            .connect_raise(clone!(@strong sender => move || {
                sender.send(Action::RaiseWindow).expect("Action channel blew up somehow");
            }));
    }
}

// #![allow(warnings)]

// use gst;
use gst::prelude::*;
use gst::ClockTime;
use gst_player;

use gtk;
use gtk::prelude::*;

use gio::{File, FileExt};
use glib::SignalHandlerId;

use crossbeam_channel::Sender;
use failure::Error;
// use send_cell::SendCell;

use hammond_data::{dbqueries, USER_AGENT};
use hammond_data::{EpisodeWidgetQuery, PodcastCoverQuery};

use app::Action;
use utils::set_image_from_path;

use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum SeekDirection {
    Backwards,
    Forward,
}

pub trait PlayerExt {
    fn play(&self);
    fn pause(&self);
    fn seek(&self, offset: ClockTime, direction: SeekDirection);
    fn fast_forward(&self);
    fn rewind(&self);
    // TODO: change playback rate
    // fn set_playback_rate(&self);
}

#[derive(Debug, Clone)]
struct PlayerInfo {
    container: gtk::Box,
    show: gtk::Label,
    episode: gtk::Label,
    cover: gtk::Image,
}

impl PlayerInfo {
    // FIXME: create a Diesel Model of the joined episode and podcast query instead
    fn init(&self, episode: &EpisodeWidgetQuery, podcast: &PodcastCoverQuery) {
        self.set_cover_image(podcast);
        self.set_show_title(podcast);
        self.set_episode_title(episode);
    }

    fn set_episode_title(&self, episode: &EpisodeWidgetQuery) {
        self.episode.set_text(&episode.title());
    }

    fn set_show_title(&self, show: &PodcastCoverQuery) {
        self.show.set_text(&show.title());
    }

    fn set_cover_image(&self, show: &PodcastCoverQuery) {
        set_image_from_path(&self.cover, show.id(), 24)
            .map_err(|err| error!("Player Cover: {}", err))
            .ok();
    }
}

#[derive(Debug, Clone)]
pub struct PlayerTimes {
    container: gtk::Box,
    progressed: gtk::Label,
    duration: gtk::Label,
    separator: gtk::Label,
    slider: gtk::Scale,
    slider_update: Arc<SignalHandlerId>,
}

#[derive(Debug, Clone, Copy)]
pub struct Duration(ClockTime);

impl Deref for Duration {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Position(ClockTime);

impl Deref for Position {
    type Target = ClockTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PlayerTimes {
    /// Update the duration `gtk::Label` and the max range of the `gtk::SclaeBar`.
    // FIXME: Refactor the labels to use some kind of Human™ time/values.
    pub fn on_duration_changed(&self, duration: Duration) {
        let seconds = duration.seconds().map(|v| v as f64).unwrap_or(0.0);

        self.slider.block_signal(&self.slider_update);
        self.slider.set_range(0.0, seconds);
        self.slider.unblock_signal(&self.slider_update);

        self.duration.set_text(&format!("{:.2}", seconds / 60.0));
    }

    /// Update the `gtk::SclaeBar` when the pipeline position is changed.
    pub fn on_position_updated(&self, position: Position) {
        let seconds = position.seconds().map(|v| v as f64).unwrap_or(0.0);

        self.slider.block_signal(&self.slider_update);
        self.slider.set_value(seconds);
        self.slider.unblock_signal(&self.slider_update);

        self.progressed.set_text(&format!("{:.2}", seconds / 60.0));
    }
}

#[derive(Debug, Clone)]
struct PlayerControls {
    container: gtk::Box,
    play: gtk::Button,
    pause: gtk::Button,
    forward: gtk::Button,
    rewind: gtk::Button,
}

#[derive(Debug, Clone)]
pub struct PlayerWidget {
    pub action_bar: gtk::ActionBar,
    player: gst_player::Player,
    controls: PlayerControls,
    pub timer: PlayerTimes,
    info: PlayerInfo,
}

impl Default for PlayerWidget {
    fn default() -> Self {
        let player = gst_player::Player::new(None, None);

        let mut config = player.get_config();
        config.set_user_agent(USER_AGENT);
        config.set_position_update_interval(250);
        config.set_seek_accurate(true);
        player.set_config(config).unwrap();

        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/player_toolbar.ui");
        let action_bar = builder.get_object("action_bar").unwrap();

        let buttons = builder.get_object("buttons").unwrap();
        let play = builder.get_object("play_button").unwrap();
        let pause = builder.get_object("pause_button").unwrap();
        let forward = builder.get_object("ff_button").unwrap();
        let rewind = builder.get_object("rewind_button").unwrap();

        let controls = PlayerControls {
            container: buttons,
            play,
            pause,
            forward,
            rewind,
        };

        let timer_container = builder.get_object("timer").unwrap();
        let progressed = builder.get_object("progress_time_label").unwrap();
        let duration = builder.get_object("total_duration_label").unwrap();
        let separator = builder.get_object("separator").unwrap();
        let slider: gtk::Scale = builder.get_object("seek").unwrap();
        slider.set_range(0.0, 1.0);
        let slider_update = Arc::new(Self::connect_update_slider(&slider, &player));
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
            container: labels,
            show,
            episode,
            cover,
        };

        PlayerWidget {
            player,
            action_bar,
            controls,
            timer,
            info,
        }
    }
}

impl PlayerWidget {
    pub fn new(sender: &Sender<Action>) -> Rc<Self> {
        let w = Rc::new(Self::default());
        Self::init(&w, sender);
        w
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn init(s: &Rc<Self>, sender: &Sender<Action>) {
        Self::connect_buttons(s);

        // Log gst warnings.
        s.player.connect_warning(move |_, warn| warn!("gst warning: {}", warn));

        // Log gst errors.
        s.player.connect_error(clone!(sender => move |_, error| {
            // FIXME: should never occur and should not be user facing.
            sender.send(Action::ErrorNotification(format!("Player Error: {}", error)))
                .map_err(|err| error!("Error: {}", err))
                .ok();

        }));

        s.player.connect_duration_changed(clone!(sender => move |_, clock| {
            sender.send(Action::PlayerDurationChanged(Duration(clock)))
                .map_err(|err| error!("Error: {}", err))
                .ok();
        }));

        s.player.connect_position_updated(clone!(sender => move |_, clock| {
            sender.send(Action::PlayerPositionUpdated(Position(clock)))
                .map_err(|err| error!("Error: {}", err))
                .ok();
        }));
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    /// Connect the `PlayerControls` buttons to the `PlayerExt` methods.
    fn connect_buttons(s: &Rc<Self>) {
        // Connect the play button to the gst Player.
        s.controls.play.connect_clicked(clone!(s => move |_| s.play()));

        // Connect the pause button to the gst Player.
        s.controls.pause.connect_clicked(clone!(s => move |_| s.pause()));

        // Connect the rewind button to the gst Player.
        s.controls.rewind.connect_clicked(clone!(s => move |_| s.rewind()));

        // Connect the fast-forward button to the gst Player.
        s.controls.forward.connect_clicked(clone!(s => move |_| s.fast_forward()));
    }

    fn reveal(&self) {
        self.action_bar.show();
    }

    pub fn initialize_episode(&self, rowid: i32) -> Result<(), Error> {
        let ep = dbqueries::get_episode_widget_from_rowid(rowid)?;
        let pd = dbqueries::get_podcast_cover_from_id(ep.podcast_id())?;

        self.info.init(&ep, &pd);
        // Currently that will always be the case since the play button is
        // only shown if the file is downloaded
        if let Some(ref path) = ep.local_uri() {
            if Path::new(path).exists() {
                // path is an absolute fs path ex. "foo/bar/baz".
                // Convert it so it will have a "file:///"
                // FIXME: convert it properly
                let uri = File::new_for_path(path).get_uri().expect("Bad file path");

                // FIXME: Maybe should also reset/flush the pipeline and then add the file?

                // play the file
                self.player.set_uri(&uri);
                self.play();
                return Ok(());
            }
            // TODO: log an error
        }

        // Stream stuff
        unimplemented!()
    }

    fn connect_update_slider(slider: &gtk::Scale, player: &gst_player::Player) -> SignalHandlerId {
        slider.connect_value_changed(clone!(player => move |slider| {
            let value = slider.get_value() as u64;
            player.seek(ClockTime::from_seconds(value as u64));
        }))
    }
}

impl PlayerExt for PlayerWidget {
    fn play(&self) {
        // assert the state is either ready or paused
        // TODO: assert!()

        self.reveal();

        self.controls.pause.show();
        self.controls.play.hide();

        self.player.play();
    }

    fn pause(&self) {
        // assert the state is paused
        // TODO: assert!()

        self.controls.pause.hide();
        self.controls.play.show();

        self.player.pause();
    }

    // Adapted from https://github.com/philn/glide/blob/b52a65d99daeab0b487f79a0e1ccfad0cd433e22/src/player_context.rs#L219-L245
    fn seek(&self, offset: ClockTime, direction: SeekDirection) {
        let position = self.player.get_position();
        if position.is_none() || offset.is_none() {
            return;
        }

        let destination = match direction {
            SeekDirection::Backwards => {
                if position >= offset {
                    Some(position - offset)
                } else {
                    None
                }
            }
            SeekDirection::Forward => {
                let duration = self.player.get_duration();
                if duration != ClockTime::none() && position + offset <= duration {
                    Some(position + offset)
                } else {
                    None
                }
            }
        };

        destination.map(|d| self.player.seek(d));
    }

    // FIXME: make the interval a GSetting
    fn rewind(&self) {
        self.seek(ClockTime::from_seconds(10), SeekDirection::Backwards)
    }

    // FIXME: make the interval a GSetting
    fn fast_forward(&self) {
        self.seek(ClockTime::from_seconds(10), SeekDirection::Forward)
    }
}

#![allow(warnings)]

use gio::{File, FileExt};

use gst::prelude::*;
use gstreamer as gst;
use gstreamer::ClockTime;
use gstreamer_player as gst_player;
use gtk;
use gtk::prelude::*;

use failure::Error;

use hammond_data::dbqueries;
use hammond_data::{EpisodeWidgetQuery, PodcastCoverQuery};

use utils::set_image_from_path;

use std::path::Path;
use std::rc::Rc;

pub trait PlayerExt {
    fn play(&self);
    fn pause(&self);
    fn seek(&self, position: ClockTime);
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
struct PlayerTimes {
    container: gtk::Box,
    progressed: gtk::Label,
    duration: gtk::Label,
    separator: gtk::Label,
    scalebar: gtk::Scale,
}

#[derive(Debug, Clone)]
// FIXME: This is a mock till stuff get sorted out.
enum PlayerState {
    Playing,
    Paused,
    Ready,
}

#[derive(Debug, Clone)]
struct PlayerControls {
    container: gtk::Box,
    play: gtk::Button,
    pause: gtk::Button,
    forward: gtk::Button,
    rewind: gtk::Button,
    // state: PlayerState,
}

#[derive(Debug, Clone)]
pub struct PlayerWidget {
    pub action_bar: gtk::ActionBar,
    player: gst_player::Player,
    controls: PlayerControls,
    timer: PlayerTimes,
    info: PlayerInfo,
}

impl Default for PlayerWidget {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/player_toolbar.ui");
        let player = gst_player::Player::new(None, None);
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
            // state: PlayerState::Ready,
        };

        let timer_container = builder.get_object("timer").unwrap();
        let progressed = builder.get_object("progress_time_label").unwrap();
        let duration = builder.get_object("total_duration_label").unwrap();
        let separator = builder.get_object("separator").unwrap();
        let scalebar: gtk::Scale = builder.get_object("seek").unwrap();
        scalebar.set_range(0.0, 1.0);
        let timer = PlayerTimes {
            container: timer_container,
            progressed,
            duration,
            separator,
            scalebar,
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
    pub fn new() -> Rc<Self> {
        let w = Rc::new(Self::default());
        Self::init(&w);
        w
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn init(s: &Rc<Self>) {
        // Connect the play button to the gst Player.
        s.controls.play.connect_clicked(clone!(s => move |_| s.play()));

        // Connect the pause button to the gst Player.
        s.controls.pause.connect_clicked(clone!(s => move |_| s.pause()));

        Self::connect_timers(s);
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

                // FIXME: Should also reset/flush the pipeline and then add the file

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

    // FIXME: Refactor to use gst_player::Player instead of raw pipeline.
    // FIXME: Refactor the labels to use some kind of Human™ time/values.
    // Adapted from https://github.com/sdroege/gstreamer-rs/blob/f4d57a66522183d4927b47af422e8f321182111f/tutorials/src/bin/basic-tutorial-5.rs#L131-L164
    fn connect_timers(s: &Rc<Self>) {
        let slider_update_signal_id = s.timer.scalebar.connect_value_changed(
            clone!(s => move |slider| {
                let pipeline = &s.player.get_pipeline();

                let value = slider.get_value() as u64;
                if let Err(_) = pipeline.seek_simple(
                    gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                    value * gst::SECOND,
                ) {
                    error!("Seeking to {} failed", value);
                }
            }),
        );

        // Update the PlayerTimes
        gtk::timeout_add_seconds(
            1,
            clone!(s => move || {
                let pipeline = s.player.get_pipeline();
                let slider = &s.timer.scalebar;

                if let Some(dur) = pipeline.query_duration::<gst::ClockTime>() {
                    let seconds = dur / gst::SECOND;
                    let seconds = seconds.map(|v| v as f64).unwrap_or(0.0);

                    slider.set_range(0.0, seconds);
                    s.timer.duration.set_text(&format!("{:.2}", seconds / 60.0));
                }

                if let Some(pos) = pipeline.query_position::<gst::ClockTime>() {
                    let seconds = pos / gst::SECOND;
                    let seconds = seconds.map(|v| v as f64).unwrap_or(0.0);

                    slider.block_signal(&slider_update_signal_id);
                    slider.set_value(seconds);
                    slider.unblock_signal(&slider_update_signal_id);

                    s.timer.progressed.set_text(&format!("{:.2}", seconds / 60.0));
                }

                Continue(true)
            }),
        );
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

    fn seek(&self, position: ClockTime) {
        self.player.seek(position);
    }

    // FIXME
    fn rewind(&self) {
        // self.seek()
    }

    // FIXME
    fn fast_forward(&self) {
        // self.seek()
    }
}

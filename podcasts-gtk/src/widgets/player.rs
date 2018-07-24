use gst::prelude::*;
use gst::ClockTime;
use gst_player;

use gtk;
use gtk::prelude::*;

use gio::{File, FileExt};
use glib::SignalHandlerId;

use chrono::NaiveTime;
use crossbeam_channel::Sender;
use failure::Error;
use send_cell::SendCell;

use podcasts_data::{dbqueries, USER_AGENT};
use podcasts_data::{EpisodeWidgetModel, ShowCoverModel};

use app::Action;
use utils::set_image_from_path;

use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;

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
}

impl PlayerInfo {
    // FIXME: create a Diesel Model of the joined episode and podcast query instead
    fn init(&self, episode: &EpisodeWidgetModel, podcast: &ShowCoverModel) {
        self.set_cover_image(podcast);
        self.set_show_title(podcast);
        self.set_episode_title(episode);
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
    pub fn on_duration_changed(&self, duration: Duration) {
        let seconds = duration.seconds().map(|v| v as f64).unwrap_or(0.0);

        self.slider.block_signal(&self.slider_update);
        self.slider.set_range(0.0, seconds);
        self.slider.unblock_signal(&self.slider_update);

        self.duration.set_text(&format_duration(seconds as u32));
    }

    /// Update the `gtk::SclaeBar` when the pipeline position is changed.
    pub fn on_position_updated(&self, position: Position) {
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
        time.format("%M:%S").to_string()
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
}

#[derive(Debug, Clone)]
pub struct PlayerWidget {
    pub action_bar: gtk::ActionBar,
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
        };

        let timer_container = builder.get_object("timer").unwrap();
        let progressed = builder.get_object("progress_time_label").unwrap();
        let duration = builder.get_object("total_duration_label").unwrap();
        let separator = builder.get_object("separator").unwrap();
        let slider: gtk::Scale = builder.get_object("seek").unwrap();
        slider.set_range(0.0, 1.0);
        let slider_update = Rc::new(Self::connect_update_slider(&slider, &player));
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
    pub fn new(sender: &Sender<Action>) -> Rc<Self> {
        let w = Rc::new(Self::default());
        Self::init(&w, sender);
        w
    }

    fn init(s: &Rc<Self>, sender: &Sender<Action>) {
        Self::connect_control_buttons(s);
        Self::connect_rate_buttons(s);
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

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn connect_gst_signals(s: &Rc<Self>, sender: &Sender<Action>) {
        // Log gst warnings.
        s.player.connect_warning(move |_, warn| warn!("gst warning: {}", warn));

        // Log gst errors.
        s.player.connect_error(clone!(sender => move |_, _error| {
            // sender.send(Action::ErrorNotification(format!("Player Error: {}", error)));
            let s = "The media player was unable to execute an action.".into();
            sender.send(Action::ErrorNotification(s));
        }));

        // The followign callbacks require `Send` but are handled by the gtk main loop
        let weak = SendCell::new(Rc::downgrade(s));

        // Update the duration label and the slider
        s.player.connect_duration_changed(clone!(weak => move |_, clock| {
            weak.borrow()
                .upgrade()
                .map(|p| p.timer.on_duration_changed(Duration(clock)));
        }));

        // Update the position label and the slider
        s.player.connect_position_updated(clone!(weak => move |_, clock| {
            weak.borrow()
                .upgrade()
                .map(|p| p.timer.on_position_updated(Position(clock)));
        }));

        // Reset the slider to 0 and show a play button
        s.player.connect_end_of_stream(clone!(weak => move |_| {
             weak.borrow()
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
        self.rate.label.set_text(&format!("{:.2}x", rate));
    }

    fn reveal(&self) {
        self.action_bar.show();
    }

    pub fn initialize_episode(&self, rowid: i32) -> Result<(), Error> {
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

    fn connect_update_slider(slider: &gtk::Scale, player: &gst_player::Player) -> SignalHandlerId {
        slider.connect_value_changed(clone!(player => move |slider| {
            let value = slider.get_value() as u64;
            player.seek(ClockTime::from_seconds(value));
        }))
    }
}

impl PlayerExt for PlayerWidget {
    fn play(&self) {
        self.reveal();

        self.controls.pause.show();
        self.controls.play.hide();

        self.player.play();
    }

    fn pause(&self) {
        self.controls.pause.hide();
        self.controls.play.show();

        self.player.pause();

        // Only rewind on pause if the stream position is passed a certain point.
        if let Some(sec) = self.player.get_position().seconds() {
            if sec >= 90 {
                self.seek(ClockTime::from_seconds(5), SeekDirection::Backwards);
            }
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    fn stop(&self) {
        self.controls.pause.hide();
        self.controls.play.show();

        self.player.stop();

        // Reset the slider bar to the start
        self.timer.on_position_updated(Position(ClockTime::from_seconds(0)));
    }

    // Adapted from https://github.com/philn/glide/blob/b52a65d99daeab0b487f79a0e1ccfad0cd433e22/src/player_context.rs#L219-L245
    fn seek(&self, offset: ClockTime, direction: SeekDirection) {
        let position = self.player.get_position();
        if position.is_none() || offset.is_none() {
            return;
        }

        let duration = self.player.get_duration();
        let destination = match direction {
            SeekDirection::Backwards if position >= offset => Some(position - offset),
            SeekDirection::Forward if !duration.is_none() && position + offset <= duration => {
                Some(position + offset)
            }
            _ => None,
        };

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

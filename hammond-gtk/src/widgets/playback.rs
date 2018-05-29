use gstreamer::ClockTime;
use gstreamer_player as gst;
use gtk;
use gtk::prelude::*;

#[derive(Debug, Clone)]
pub struct Playback {
    reveal: gtk::Revealer,
    container: gtk::Grid,
    play: gtk::Button,
    seek: gtk::Scale,
    title: gtk::Label,
    time: gtk::Label,
}

impl Default for Playback {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/playback.ui");
        let container = builder.get_object("wrapper").unwrap();
        let play = builder.get_object("play").unwrap();
        let seek = builder.get_object("seek").unwrap();
        let title = builder.get_object("title").unwrap();
        let time = builder.get_object("time").unwrap();

        let reveal = gtk::Revealer::new();
        reveal.set_no_show_all(true);
        reveal.add(&container);

        Playback { reveal, container, play, seek, title, time }
    }
}

impl Playback {
    pub fn new() -> Playback {
        Playback::default()
    }

    pub fn set_icon(&self, icon: &str) {
        let image = gtk::Image::new_from_icon_name(icon, gtk::IconSize::Button.into());
        self.play.set_image(Some(&image));
    }

    pub fn get_widget(&self) -> &gtk::Revealer {
        &self.reveal
    }

    pub fn state_changed(&self, state: gst::PlayerState) {
        // Once the playback controls are shown they don't go
        // away again so show them unconditionally
        self.reveal.show();
        self.reveal.set_reveal_child(true);
        match state {
            gst::PlayerState::Buffering => {
                println!("Buffering!!!!!!!");
            },
            gst::PlayerState::Stopped | gst::PlayerState::Paused => {
                println!("Stopped/Paused");
                self.set_icon("media-playback-start-symbolic");
            },
            gst::PlayerState::Playing => {
                println!("Playing");
                self.set_icon("media-playback-pause-symbolic");
            },
            _ => {
                println!("Weird stuff");
            }
        }
    }

    pub fn media_changed(&self, title: Option<String>, length: ClockTime) {
        self.reveal.show();
        self.reveal.set_reveal_child(true);
        if let Some(title) = title {
            self.title.set_label(&title);
        } else {
            self.title.set_label("");
        }
        if let Some(s) = length.seconds() {
            let hours = s / 3600;
            let s = s - (hours * 3600);
            let mins = s / 60;
            let s = s - (mins * 60);
            // This is a little nasty
            let t = format!("{}:{}:{}", hours, mins, s);
            self.time.set_label(&t);
        } else {
            self.time.set_label("");
        }
    }

    pub fn position_changed(&self, _pos: ClockTime) {
        println!("Tada!");
    }
}

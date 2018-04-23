use gtk;
use gtk::prelude::*;
use gtk::Orientation::Vertical;
use relm::{Relm, Update, Widget};

use chrono::prelude::*;

#[derive(Msg)]
enum TitleMsg {
    Normal,
    GreyedOut,
    SetText(String),
}

// Create the structure that holds the widgets used in the view.
struct Title {
    title: gtk::Label,
}

impl Update for Title {
    // Specify the model used for this widget.
    type Model = ();
    // Specify the model parameter used to init the model.
    type ModelParam = ();
    // Specify the type of the messages sent to the update function.
    type Msg = TitleMsg;

    fn model(_: &Relm<Self>, _: ()) -> () {}

    fn update(&mut self, event: TitleMsg) {
        match event {
            TitleMsg::Normal => {
                self.title
                    .get_style_context()
                    .map(|c| c.remove_class("dim-label"));
            }
            TitleMsg::GreyedOut => {
                self.title
                    .get_style_context()
                    .map(|c| c.add_class("dim-label"));
            }
            TitleMsg::SetText(s) => {
                self.title.set_text(&s);
                // self.title.set_tooltip_text(s.as_str());
            }
        }
    }
}

impl Widget for Title {
    // Specify the type of the root widget.
    type Root = gtk::Label;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.title.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let title = gtk::Label::new("Unkown Title");

        Title { title }
    }
}

#[derive(Msg)]
enum DateMsg {
    Usual,
    ShowYear,
}

struct Date {
    date: gtk::Label,
    epoch: i64
}

impl Update for Date {
    type Model = i64;
    type ModelParam = i64;
    type Msg = DateMsg;

    fn model(_: &Relm<Self>, epoch: i64) -> i64 {
        epoch
    }

    fn update(&mut self, event: DateMsg) {
        match event {
            DateMsg::Usual => {
                let ts = Utc.timestamp(self.epoch, 0);
                self.date.set_text(ts.format("%e %b").to_string().trim());
            }
            DateMsg::ShowYear => {
                let ts = Utc.timestamp(self.epoch, 0);
                self.date.set_text(ts.format("%e %b %Y").to_string().trim());
            }
        }
    }
}

impl Widget for Date {
    type Root = gtk::Label;

    fn root(&self) -> Self::Root {
        self.date.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let date = gtk::Label::new("");

        Date { date, epoch: model }
    }
}

#[derive(Msg)]
enum DurationMsg {
    Show,
    Hide,
    SetDuration,
}

struct Duration {
    duration: gtk::Label,
    separator: gtk::Label,
    seconds: Option<i32>,
}

impl Update for Date {
    type Model = Option<i32>;
    type ModelParam = Option<i32>;
    type Msg = DateMsg;

    fn model(_: &Relm<Self>, seconds: Option<i32>) -> Option<i32> {
        seconds
    }

    fn update(&mut self, event: DateMsg) {
        match event {
            DurationMsg::Show => {
                self.duration.show();
                self.separator.show();
            }
            DurationMsg::Hide => {
                self.duration.hide();
                self.separator.hide();
            }
            DurationMsg::SetDuration => {
                let minutes = chrono::Duration::seconds(self.into()).num_minutes();
                if miutes == 0 {
                    // FIXME: emit DurationMsg::Hide
                } else {
                    // FIXME: emit DurationMsg::Show
                    self.duration.set_text(&format!("{} min", minutes));
                }
            }
        }
    }
}

impl Widget for Date {
    // FIXME: This is weird
    type Root = gtk::Label;

    fn root(&self) -> Self::Root {
        self.date.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let date = gtk::Label::new("");

        Date { date, epoch: model }
    }
}


// struct Model {
//     counter: i32,
// }

// #[derive(Msg)]
// enum Msg {
//     Decrement,
//     Increment,
// }

// Create the structure that holds the widgets used in the view.
// struct EpisodeWidgetRelm {
//     model: Model,

//     container: gtk::Box,
//     progress: gtk::ProgressBar,

//     download: gtk::Button,
//     play: gtk::Button,
//     cancel: gtk::Button,

//     title: gtk::Label,
//     date: gtk::Label,
//     duration: gtk::Label,
//     local_size: gtk::Label,
//     total_size: gtk::Label,

//     separator1: gtk::Label,
//     separator2: gtk::Label,
//     prog_separator: gtk::Label,
// }

// impl Update for EpisodeWidgetRelm {
    // Specify the model used for this widget.
//     type Model = Model;
    // Specify the model parameter used to init the model.
//     type ModelParam = ();
    // Specify the type of the messages sent to the update function.
//     type Msg = Msg;

//     fn model(_: &Relm<Self>, _: ()) -> Model {
//         Model { counter: 0 }
//     }

//     fn update(&mut self, event: Msg) {
//         match event {
//             Msg::Decrement => {}
//             Msg::Increment => {}
//         }
//     }
// }

// impl Widget for EpisodeWidgetRelm {
    // Specify the type of the root widget.
//     type Root = gtk::Box;

    // Return the root widget.
//     fn root(&self) -> Self::Root {
//         self.container.clone()
//     }

//     fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
//         let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/episode_widget.ui");

//         let container: gtk::Box = builder.get_object("episode_container").unwrap();
//         let progress: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();

//         let download: gtk::Button = builder.get_object("download_button").unwrap();
//         let play: gtk::Button = builder.get_object("play_button").unwrap();
//         let cancel: gtk::Button = builder.get_object("cancel_button").unwrap();

//         let title: gtk::Label = builder.get_object("title_label").unwrap();
//         let date: gtk::Label = builder.get_object("date_label").unwrap();
//         let duration: gtk::Label = builder.get_object("duration_label").unwrap();
//         let local_size: gtk::Label = builder.get_object("local_size").unwrap();
//         let total_size: gtk::Label = builder.get_object("total_size").unwrap();

//         let separator1: gtk::Label = builder.get_object("separator1").unwrap();
//         let separator2: gtk::Label = builder.get_object("separator2").unwrap();
//         let prog_separator: gtk::Label = builder.get_object("prog_separator").unwrap();

        // Send the message Increment when the button is clicked.
        // connect!(relm, plus_button, connect_clicked(_), Msg::Increment);
        // connect!(relm, minus_button, connect_clicked(_), Msg::Decrement);

//         EpisodeWidgetRelm {
//             model,
//             container,
//             progress,

//             download,
//             play,
//             cancel,

//             title,
//             date,
//             duration,
//             local_size,
//             total_size,

//             separator1,
//             separator2,
//             prog_separator,
//         }
//     }
// }
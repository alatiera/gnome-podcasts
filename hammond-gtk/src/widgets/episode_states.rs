use chrono;
use gtk;
use gtk::prelude::*;

#[derive(Debug, Clone)]
pub struct Normal;
#[derive(Debug, Clone)]
pub struct GreyedOut;

#[derive(Debug, Clone)]
pub struct Title<S> {
    title: gtk::Label,
    state: S,
}

impl<S> Title<S> {
    #[allow(unused_must_use)]
    // This does not need to be &mut since gtk-rs does not model ownership
    // But I think it wouldn't heart if we treat it as a Rust api.
    fn set_title(&mut self, s: &str) {
        self.title.set_text(s);
    }
}

impl Title<Normal> {
    fn new(title: gtk::Label) -> Self {
        Title {
            title,
            state: Normal {},
        }
    }
}

impl From<Title<Normal>> for Title<GreyedOut> {
    fn from(machine: Title<Normal>) -> Self {
        machine
            .title
            .get_style_context()
            .map(|c| c.add_class("dim-label"));

        Title {
            title: machine.title,
            state: GreyedOut {},
        }
    }
}

impl From<Title<GreyedOut>> for Title<Normal> {
    fn from(machine: Title<GreyedOut>) -> Self {
        machine
            .title
            .get_style_context()
            .map(|c| c.remove_class("dim-label"));

        Title {
            title: machine.title,
            state: Normal {},
        }
    }
}

#[derive(Debug, Clone)]
pub enum TitleMachine {
    Normal(Title<Normal>),
    GreyedOut(Title<GreyedOut>),
}

impl TitleMachine {
    pub fn new(label: gtk::Label, is_played: bool) -> Self {
        let m = TitleMachine::Normal(Title::<Normal>::new(label));
        m.determine_state(is_played)
    }

    pub fn determine_state(self, is_played: bool) -> Self {
        match (self, is_played) {
            (title @ TitleMachine::Normal(_), false) => title,
            (title @ TitleMachine::GreyedOut(_), true) => title,
            (TitleMachine::Normal(val), true) => TitleMachine::GreyedOut(val.into()),
            (TitleMachine::GreyedOut(val), false) => TitleMachine::Normal(val.into()),
        }
    }

    pub fn set_title(&mut self, s: &str) {
        match *self {
            TitleMachine::Normal(ref mut val) => val.set_title(s),
            TitleMachine::GreyedOut(ref mut val) => val.set_title(s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Shown;
#[derive(Debug, Clone)]
pub struct Hidden;

#[derive(Debug, Clone)]
pub struct Duration<S> {
    // TODO: make duration and separator diff types
    duration: gtk::Label,
    separator: gtk::Label,
    state: S,
}

impl<S> Duration<S> {
    // This needs a better name.
    fn set_duration(&self, minutes: i64) {
        self.duration.set_text(&format!("{} min", minutes));
    }
}

impl Duration<Hidden> {
    fn new(duration: gtk::Label, separator: gtk::Label) -> Self {
        duration.hide();
        separator.hide();

        Duration {
            duration,
            separator,
            state: Hidden {},
        }
    }
}

impl From<Duration<Hidden>> for Duration<Shown> {
    fn from(d: Duration<Hidden>) -> Self {
        d.duration.show();
        d.separator.show();

        Duration {
            duration: d.duration,
            separator: d.separator,
            state: Shown {},
        }
    }
}

impl From<Duration<Shown>> for Duration<Hidden> {
    fn from(d: Duration<Shown>) -> Self {
        d.duration.hide();
        d.separator.hide();

        Duration {
            duration: d.duration,
            separator: d.separator,
            state: Hidden {},
        }
    }
}

#[derive(Debug, Clone)]
pub enum DurationMachine {
    Hidden(Duration<Hidden>),
    Shown(Duration<Shown>),
}

impl DurationMachine {
    pub fn new(duration: gtk::Label, separator: gtk::Label, seconds: Option<i32>) -> Self {
        let m = DurationMachine::Hidden(Duration::<Hidden>::new(duration, separator));
        m.determine_state(seconds)
    }

    pub fn determine_state(self, seconds: Option<i32>) -> Self {
        match (self, seconds) {
            (DurationMachine::Hidden(val), None) => DurationMachine::Hidden(val.into()),
            (DurationMachine::Shown(val), None) => DurationMachine::Hidden(val.into()),
            (DurationMachine::Hidden(val), Some(s)) => {
                let minutes = chrono::Duration::seconds(s.into()).num_minutes();
                if minutes == 0 {
                    DurationMachine::Hidden(val.into())
                } else {
                    val.set_duration(minutes);
                    DurationMachine::Shown(val.into())
                }
            }
            (DurationMachine::Shown(val), Some(s)) => {
                let minutes = chrono::Duration::seconds(s.into()).num_minutes();
                if minutes == 0 {
                    DurationMachine::Hidden(val.into())
                } else {
                    val.set_duration(minutes);
                    DurationMachine::Shown(val.into())
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalShown;

#[derive(Debug, Clone)]
pub struct TotalShown;

#[derive(Debug, Clone)]
pub struct Unkown;

#[derive(Debug, Clone)]
pub struct InProgress;

#[derive(Debug, Clone)]
pub struct Size<S> {
    local_size: gtk::Label,
    total_size: gtk::Label,
    separator: gtk::Label,
    prog_separator: gtk::Label,
    state: S,
}

impl Size<Unkown> {
    fn new(
        local_size: gtk::Label,
        total_size: gtk::Label,
        separator: gtk::Label,
        prog_separator: gtk::Label,
    ) -> Self {
        local_size.hide();
        total_size.hide();
        separator.hide();
        prog_separator.hide();

        Size {
            local_size,
            total_size,
            separator,
            prog_separator,
            state: Unkown {},
        }
    }
}

impl From<Size<TotalShown>> for Size<LocalShown> {
    fn from(f: Size<TotalShown>) -> Self {
        f.prog_separator.hide();
        f.total_size.hide();
        f.local_size.show();
        f.separator.show();

        Size {
            local_size: f.local_size,
            total_size: f.total_size,
            separator: f.separator,
            prog_separator: f.prog_separator,
            state: LocalShown {},
        }
    }
}

impl From<Size<TotalShown>> for Size<InProgress> {
    fn from(f: Size<TotalShown>) -> Self {
        f.prog_separator.show();
        f.total_size.show();
        f.local_size.show();
        f.separator.show();

        Size {
            local_size: f.local_size,
            total_size: f.total_size,
            separator: f.separator,
            prog_separator: f.prog_separator,
            state: InProgress {},
        }
    }
}

impl From<Size<Unkown>> for Size<InProgress> {
    fn from(f: Size<Unkown>) -> Self {
        f.prog_separator.show();
        f.total_size.show();
        f.local_size.show();
        f.separator.show();

        Size {
            local_size: f.local_size,
            total_size: f.total_size,
            separator: f.separator,
            prog_separator: f.prog_separator,
            state: InProgress {},
        }
    }
}

impl From<Size<InProgress>> for Size<LocalShown> {
    fn from(f: Size<InProgress>) -> Self {
        f.prog_separator.hide();
        f.total_size.hide();
        f.local_size.show();
        f.separator.show();

        Size {
            local_size: f.local_size,
            total_size: f.total_size,
            separator: f.separator,
            prog_separator: f.prog_separator,
            state: LocalShown {},
        }
    }
}

pub enum SizeMachine {
    LocalShown(Size<LocalShown>),
    TotallShown(Size<TotalShown>),
    Unkown(Size<Unkown>),
    InProgress(Size<InProgress>),
}

impl SizeMachine {
    pub fn new(
        local_size: gtk::Label,
        total_size: gtk::Label,
        separator: gtk::Label,
        prog_separator: gtk::Label,
    ) -> Self {
        SizeMachine::Unkown(Size::<Unkown>::new(
            local_size,
            total_size,
            separator,
            prog_separator,
        ))
    }

    pub fn determine_state(self) -> Self {
        unimplemented!()
    }
}

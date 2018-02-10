//  TODO: Things that should be done.
//
// * Wherever there's a function that take 2 or more arguments of the same type,
//   eg: fn new(total_size: gtk::Label, local_size: gtk::Label ..)
//   Wrap the types into Struct-tuples and imple deref so it won't be possible to pass
//   the wrong argument to the wrong position.

use chrono;
use gtk;
use gtk::prelude::*;

#[derive(Debug, Clone)]
pub struct Shown;
#[derive(Debug, Clone)]
pub struct Hidden;

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
    fn from(f: Duration<Hidden>) -> Self {
        f.duration.show();
        f.separator.show();

        Duration {
            duration: f.duration,
            separator: f.separator,
            state: Shown {},
        }
    }
}

impl From<Duration<Shown>> for Duration<Hidden> {
    fn from(f: Duration<Shown>) -> Self {
        f.duration.hide();
        f.separator.hide();

        Duration {
            duration: f.duration,
            separator: f.separator,
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Download;

#[derive(Debug, Clone)]
pub struct Play;

#[derive(Debug, Clone)]
// FIXME: Needs better name.
// Should each button also has it's own type and machine?
pub struct DownloadPlay<S> {
    play: gtk::Button,
    download: gtk::Button,
    state: S,
}

impl DownloadPlay<Hidden> {
    fn new(play: gtk::Button, download: gtk::Button) -> Self {
        play.hide();
        download.hide();

        DownloadPlay {
            play,
            download,
            state: Hidden {},
        }
    }
}

impl From<DownloadPlay<Play>> for DownloadPlay<Download> {
    fn from(f: DownloadPlay<Play>) -> Self {
        f.play.hide();
        f.download.show();

        DownloadPlay {
            play: f.play,
            download: f.download,
            state: Download {},
        }
    }
}

impl From<DownloadPlay<Download>> for DownloadPlay<Play> {
    fn from(f: DownloadPlay<Download>) -> Self {
        f.play.show();
        f.download.hide();

        DownloadPlay {
            play: f.play,
            download: f.download,
            state: Play {},
        }
    }
}

impl From<DownloadPlay<Play>> for DownloadPlay<Hidden> {
    fn from(f: DownloadPlay<Play>) -> Self {
        f.play.hide();
        f.download.hide();

        DownloadPlay {
            play: f.play,
            download: f.download,
            state: Hidden {},
        }
    }
}

impl From<DownloadPlay<Download>> for DownloadPlay<Hidden> {
    fn from(f: DownloadPlay<Download>) -> Self {
        f.play.hide();
        f.download.hide();

        DownloadPlay {
            play: f.play,
            download: f.download,
            state: Hidden {},
        }
    }
}

impl From<DownloadPlay<Hidden>> for DownloadPlay<Download> {
    fn from(f: DownloadPlay<Hidden>) -> Self {
        f.play.hide();
        f.download.show();

        DownloadPlay {
            play: f.play,
            download: f.download,
            state: Download {},
        }
    }
}

impl From<DownloadPlay<Hidden>> for DownloadPlay<Play> {
    fn from(f: DownloadPlay<Hidden>) -> Self {
        f.play.show();
        f.download.show();

        DownloadPlay {
            play: f.play,
            download: f.download,
            state: Play {},
        }
    }
}

pub enum DownloadPlayMachine {
    Play(DownloadPlay<Play>),
    Download(DownloadPlay<Download>),
    Hidden(DownloadPlay<Hidden>),
}

impl DownloadPlayMachine {
    pub fn new(play: gtk::Button, download: gtk::Button) -> Self {
        DownloadPlayMachine::Hidden(DownloadPlay::<Hidden>::new(play, download))
    }

    pub fn determine_state(self, downloaded: bool, should_hide: bool) -> Self {
        match (self, downloaded, should_hide) {
            (DownloadPlayMachine::Play(val), true, false) => DownloadPlayMachine::Play(val.into()),
            (DownloadPlayMachine::Play(val), false, false) => {
                DownloadPlayMachine::Download(val.into())
            }
            (DownloadPlayMachine::Download(val), true, false) => {
                DownloadPlayMachine::Play(val.into())
            }
            (DownloadPlayMachine::Download(val), false, false) => {
                DownloadPlayMachine::Download(val.into())
            }
            (DownloadPlayMachine::Hidden(val), true, false) => {
                DownloadPlayMachine::Play(val.into())
            }
            (DownloadPlayMachine::Hidden(val), false, false) => {
                DownloadPlayMachine::Download(val.into())
            }
            (DownloadPlayMachine::Play(val), _, true) => DownloadPlayMachine::Hidden(val.into()),
            (DownloadPlayMachine::Download(val), _, true) => {
                DownloadPlayMachine::Hidden(val.into())
            }
            (DownloadPlayMachine::Hidden(val), _, true) => DownloadPlayMachine::Hidden(val.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Progress<S> {
    bar: gtk::ProgressBar,
    cancel: gtk::Button,
    state: S,
}

impl Progress<Hidden> {
    fn new(bar: gtk::ProgressBar, cancel: gtk::Button) -> Self {
        bar.hide();
        cancel.hide();

        Progress {
            bar,
            cancel,
            state: Hidden {},
        }
    }
}

impl From<Progress<Hidden>> for Progress<Shown> {
    fn from(f: Progress<Hidden>) -> Self {
        f.bar.show();
        f.cancel.show();

        Progress {
            bar: f.bar,
            cancel: f.cancel,
            state: Shown {},
        }
    }
}

impl From<Progress<Shown>> for Progress<Hidden> {
    fn from(f: Progress<Shown>) -> Self {
        f.bar.hide();
        f.cancel.hide();

        Progress {
            bar: f.bar,
            cancel: f.cancel,
            state: Hidden {},
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProgressMachine {
    Hidden(Progress<Hidden>),
    Shown(Progress<Shown>),
}

impl ProgressMachine {
    pub fn new(bar: gtk::ProgressBar, cancel: gtk::Button) -> Self {
        ProgressMachine::Hidden(Progress::<Hidden>::new(bar, cancel))
    }

    pub fn determine_state(self, is_active: bool) -> Self {
        match (self, is_active) {
            (ProgressMachine::Hidden(val), false) => ProgressMachine::Hidden(val.into()),
            (ProgressMachine::Hidden(val), true) => ProgressMachine::Shown(val.into()),
            (ProgressMachine::Shown(val), false) => ProgressMachine::Hidden(val.into()),
            (ProgressMachine::Shown(val), true) => ProgressMachine::Shown(val.into()),
        }
    }
}

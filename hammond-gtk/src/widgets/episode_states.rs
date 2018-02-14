//  TODO: Things that should be done.
//
// * Wherever there's a function that take 2 or more arguments of the same type,
//   eg: fn new(total_size: gtk::Label, local_size: gtk::Label ..)
//   Wrap the types into Struct-tuples and imple deref so it won't be possible to pass
//   the wrong argument to the wrong position.

use chrono;
use gtk;
use gtk::prelude::*;

pub trait Visibility {}

#[derive(Debug, Clone)]
pub struct UnItialized;

#[derive(Debug, Clone)]
pub struct Shown;
#[derive(Debug, Clone)]
pub struct Hidden;

impl Visibility for Shown {}
impl Visibility for Hidden {}

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
pub struct Duration<S: Visibility> {
    // TODO: make duration and separator diff types
    duration: gtk::Label,
    separator: gtk::Label,
    state: S,
}

impl<S: Visibility> Duration<S> {
    // This needs a better name.
    // TODO: make me mut
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
pub struct Size<S> {
    size: gtk::Label,
    separator: gtk::Label,
    state: S,
}

impl Size<Shown> {
    fn set_size(self, s: &str) -> Size<Shown> {
        self.size.set_text(s);
        self.separator.show();
        self.into()
    }
}

impl Size<Hidden> {
    fn set_size(self, s: &str) -> Size<Shown> {
        self.size.set_text(s);
        self.separator.show();
        self.into()
    }
}

impl Size<UnItialized> {
    fn new(size: gtk::Label, separator: gtk::Label) -> Self {
        size.hide();
        separator.hide();

        Size {
            size,
            separator,
            state: UnItialized {},
        }
    }

    fn set_size(self, s: &str) -> Size<Shown> {
        self.size.set_text(s);
        self.separator.show();
        self.into()
    }
}

impl From<Size<Shown>> for Size<Hidden> {
    fn from(f: Size<Shown>) -> Self {
        f.size.hide();
        f.separator.hide();

        Size {
            size: f.size,
            separator: f.separator,
            state: Hidden {},
        }
    }
}

impl From<Size<Hidden>> for Size<Shown> {
    fn from(f: Size<Hidden>) -> Self {
        f.size.show();
        f.separator.show();

        Size {
            size: f.size,
            separator: f.separator,
            state: Shown {},
        }
    }
}

impl From<Size<UnItialized>> for Size<Shown> {
    /// This is suposed to be called only from Size::<UnInitialize>::set_size.
    fn from(f: Size<UnItialized>) -> Self {
        f.size.show();
        f.separator.show();

        Size {
            size: f.size,
            separator: f.separator,
            state: Shown {},
        }
    }
}

impl From<Size<UnItialized>> for Size<Hidden> {
    /// This is suposed to be called only from Size::<UnInitialized>::set_size.
    fn from(f: Size<UnItialized>) -> Self {
        f.size.hide();
        f.separator.hide();

        Size {
            size: f.size,
            separator: f.separator,
            state: Hidden {},
        }
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

impl DownloadPlay<UnItialized> {
    fn new(play: gtk::Button, download: gtk::Button) -> Self {
        play.hide();
        download.hide();

        DownloadPlay {
            play,
            download,
            state: UnItialized {},
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

impl From<DownloadPlay<UnItialized>> for DownloadPlay<Download> {
    fn from(f: DownloadPlay<UnItialized>) -> Self {
        f.play.hide();
        f.download.show();

        DownloadPlay {
            play: f.play,
            download: f.download,
            state: Download {},
        }
    }
}

impl From<DownloadPlay<UnItialized>> for DownloadPlay<Play> {
    fn from(f: DownloadPlay<UnItialized>) -> Self {
        f.play.show();
        f.download.show();

        DownloadPlay {
            play: f.play,
            download: f.download,
            state: Play {},
        }
    }
}

impl From<DownloadPlay<UnItialized>> for DownloadPlay<Hidden> {
    fn from(f: DownloadPlay<UnItialized>) -> Self {
        f.play.hide();
        f.download.hide();

        DownloadPlay {
            play: f.play,
            download: f.download,
            state: Hidden {},
        }
    }
}

#[derive(Debug, Clone)]
pub struct Progress<S> {
    bar: gtk::ProgressBar,
    cancel: gtk::Button,
    local_size: gtk::Label,
    prog_separator: gtk::Label,
    state: S,
}

impl Progress<UnItialized> {
    fn new(
        bar: gtk::ProgressBar,
        cancel: gtk::Button,
        local_size: gtk::Label,
        prog_separator: gtk::Label,
    ) -> Self {
        bar.hide();
        cancel.hide();
        local_size.hide();
        prog_separator.hide();

        Progress {
            bar,
            cancel,
            local_size,
            prog_separator,
            state: UnItialized {},
        }
    }
}

impl From<Progress<Hidden>> for Progress<Shown> {
    fn from(f: Progress<Hidden>) -> Self {
        f.bar.show();
        f.cancel.show();
        f.local_size.show();
        f.prog_separator.show();

        Progress {
            bar: f.bar,
            cancel: f.cancel,
            local_size: f.local_size,
            prog_separator: f.prog_separator,
            state: Shown {},
        }
    }
}

impl From<Progress<Shown>> for Progress<Hidden> {
    fn from(f: Progress<Shown>) -> Self {
        f.bar.hide();
        f.cancel.hide();
        f.local_size.hide();
        f.prog_separator.hide();

        Progress {
            bar: f.bar,
            cancel: f.cancel,
            local_size: f.local_size,
            prog_separator: f.prog_separator,
            state: Hidden {},
        }
    }
}

impl From<Progress<UnItialized>> for Progress<Shown> {
    fn from(f: Progress<UnItialized>) -> Self {
        f.bar.show();
        f.cancel.show();
        f.local_size.show();
        f.prog_separator.show();

        Progress {
            bar: f.bar,
            cancel: f.cancel,
            local_size: f.local_size,
            prog_separator: f.prog_separator,
            state: Shown {},
        }
    }
}

impl From<Progress<UnItialized>> for Progress<Hidden> {
    fn from(f: Progress<UnItialized>) -> Self {
        f.bar.hide();
        f.cancel.hide();
        f.local_size.hide();
        f.prog_separator.hide();

        Progress {
            bar: f.bar,
            cancel: f.cancel,
            local_size: f.local_size,
            prog_separator: f.prog_separator,
            state: Hidden {},
        }
    }
}

#[derive(Debug, Clone)]
pub struct Media<X, Z, Y> {
    dl: DownloadPlay<X>,
    total_size: Size<Z>,
    progress: Progress<Y>,
}

// From New fro InProgress
impl From<Media<Download, Shown, Hidden>> for Media<Hidden, Shown, Shown> {
    fn from(f: Media<Download, Shown, Hidden>) -> Self {
        Media {
            dl: f.dl.into(),
            total_size: f.total_size.into(),
            progress: f.progress.into(),
        }
    }
}

// From NewWithoutSize fro InProgress
impl From<Media<Download, Hidden, Hidden>> for Media<Hidden, Shown, Shown> {
    fn from(f: Media<Download, Hidden, Hidden>) -> Self {
        Media {
            dl: f.dl.into(),
            total_size: f.total_size.into(),
            progress: f.progress.into(),
        }
    }
}

// Into New
impl Into<Media<Download, Hidden, Shown>> for Media<UnItialized, UnItialized, UnItialized> {
    fn into(self) -> Media<Download, Hidden, Shown> {
        Media {
            dl: self.dl.into(),
            total_size: self.total_size.into(),
            progress: self.progress.into(),
        }
    }
}

// Into NewWithoutSize
impl Into<Media<Download, Hidden, Hidden>> for Media<UnItialized, UnItialized, UnItialized> {
    fn into(self) -> Media<Download, Hidden, Hidden> {
        Media {
            dl: self.dl.into(),
            total_size: self.total_size.into(),
            progress: self.progress.into(),
        }
    }
}

// Into Playable
impl Into<Media<Play, Hidden, Shown>> for Media<UnItialized, UnItialized, UnItialized> {
    fn into(self) -> Media<Play, Hidden, Shown> {
        Media {
            dl: self.dl.into(),
            total_size: self.total_size.into(),
            progress: self.progress.into(),
        }
    }
}

// Into PlayableWithoutSize
impl Into<Media<Play, Hidden, Hidden>> for Media<UnItialized, UnItialized, UnItialized> {
    fn into(self) -> Media<Play, Hidden, Hidden> {
        Media {
            dl: self.dl.into(),
            total_size: self.total_size.into(),
            progress: self.progress.into(),
        }
    }
}

// Into InProgress
impl Into<Media<Hidden, Shown, Shown>> for Media<UnItialized, UnItialized, UnItialized> {
    fn into(self) -> Media<Hidden, Shown, Shown> {
        Media {
            dl: self.dl.into(),
            total_size: self.total_size.into(),
            progress: self.progress.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MediaMachine {
    New(Media<Download, Hidden, Shown>),
    NewWithoutSize(Media<Download, Hidden, Hidden>),
    Playable(Media<Play, Hidden, Shown>),
    PlayableWithoutSize(Media<Play, Hidden, Hidden>),
    InProgress(Media<Hidden, Shown, Shown>),
}

#[derive(Debug, Clone)]
pub enum MediaMachineWrapper {
    UnItialized(Media<UnItialized, UnItialized, UnItialized>),
    Initialized(MediaMachine),
}

impl MediaMachineWrapper {
    pub fn new(
        play: gtk::Button,
        download: gtk::Button,
        bar: gtk::ProgressBar,
        cancel: gtk::Button,
        total_size: gtk::Label,
        local_size: gtk::Label,
        separator: gtk::Label,
        prog_separator: gtk::Label,
    ) -> Self {
        let dl = DownloadPlay::<UnItialized>::new(play, download);
        let progress = Progress::<UnItialized>::new(bar, cancel, local_size, prog_separator);
        let total_size = Size::<UnItialized>::new(total_size, separator);

        MediaMachineWrapper::UnItialized(Media {
            dl,
            progress,
            total_size,
        })
    }
}

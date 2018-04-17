//  TODO: Things that should be done.
//
// * Wherever there's a function that take 2 or more arguments of the same type,
//   eg: fn new(total_size: gtk::Label, local_size: gtk::Label ..)
//   Wrap the types into Struct-tuples and imple deref so it won't be possible to pass
//   the wrong argument to the wrong position.

use chrono;
use glib;
use gtk;

use chrono::prelude::*;
use gtk::prelude::*;
use humansize::{file_size_opts as size_opts, FileSize};

use std::sync::Arc;

lazy_static! {
    pub static ref SIZE_OPTS: Arc<size_opts::FileSizeOpts> =  {
        // Declare a custom humansize option struct
        // See: https://docs.rs/humansize/1.0.2/humansize/file_size_opts/struct.FileSizeOpts.html
        Arc::new(size_opts::FileSizeOpts {
            divider: size_opts::Kilo::Binary,
            units: size_opts::Kilo::Decimal,
            decimal_places: 0,
            decimal_zeroes: 0,
            fixed_at: size_opts::FixedAt::No,
            long_units: false,
            space: true,
            suffix: "",
            allow_negative: false,
        })
    };

    static ref NOW: DateTime<Utc> = Utc::now();
}

#[derive(Debug, Clone)]
pub struct UnInitialized;

#[derive(Debug, Clone)]
pub struct Shown;
#[derive(Debug, Clone)]
pub struct Hidden;

pub trait Visibility {}

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
    #[inline]
    // This does not need to be &mut since gtk-rs does not model ownership
    // But I think it wouldn't hurt if we treat it as a Rust api.
    fn set_title(&mut self, s: &str) {
        self.title.set_text(s);
    }
}

impl Title<Normal> {
    #[inline]
    fn new(title: gtk::Label) -> Self {
        Title {
            title,
            state: Normal {},
        }
    }
}

impl From<Title<Normal>> for Title<GreyedOut> {
    #[inline]
    fn from(f: Title<Normal>) -> Self {
        f.title
            .get_style_context()
            .map(|c| c.add_class("dim-label"));

        Title {
            title: f.title,
            state: GreyedOut {},
        }
    }
}

impl From<Title<GreyedOut>> for Title<Normal> {
    #[inline]
    fn from(f: Title<GreyedOut>) -> Self {
        f.title
            .get_style_context()
            .map(|c| c.remove_class("dim-label"));

        Title {
            title: f.title,
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
    #[inline]
    pub fn new(label: gtk::Label, is_played: bool) -> Self {
        let m = TitleMachine::Normal(Title::<Normal>::new(label));
        m.determine_state(is_played)
    }

    #[inline]
    pub fn determine_state(self, is_played: bool) -> Self {
        use self::TitleMachine::*;

        match (self, is_played) {
            (title @ Normal(_), false) => title,
            (title @ GreyedOut(_), true) => title,
            (Normal(val), true) => GreyedOut(val.into()),
            (GreyedOut(val), false) => Normal(val.into()),
        }
    }

    #[inline]
    pub fn set_title(&mut self, s: &str) {
        use self::TitleMachine::*;

        match *self {
            Normal(ref mut val) => val.set_title(s),
            GreyedOut(ref mut val) => val.set_title(s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Usual;
#[derive(Debug, Clone)]
pub struct YearShown;

#[derive(Debug, Clone)]
pub struct Date<S> {
    date: gtk::Label,
    epoch: i64,
    state: S,
}

impl<S> Date<S> {
    #[inline]
    fn into_usual(self, epoch: i64) -> Date<Usual> {
        let ts = Utc.timestamp(epoch, 0);
        self.date.set_text(ts.format("%e %b").to_string().trim());

        Date {
            date: self.date,
            epoch: self.epoch,
            state: Usual {},
        }
    }

    #[inline]
    fn into_year_shown(self, epoch: i64) -> Date<YearShown> {
        let ts = Utc.timestamp(epoch, 0);
        self.date.set_text(ts.format("%e %b %Y").to_string().trim());

        Date {
            date: self.date,
            epoch: self.epoch,
            state: YearShown {},
        }
    }
}

impl Date<UnInitialized> {
    #[inline]
    fn new(date: gtk::Label, epoch: i64) -> Self {
        let ts = Utc.timestamp(epoch, 0);
        date.set_text(ts.format("%e %b %Y").to_string().trim());

        Date {
            date,
            epoch,
            state: UnInitialized {},
        }
    }
}

#[derive(Debug, Clone)]
pub enum DateMachine {
    UnInitialized(Date<UnInitialized>),
    Usual(Date<Usual>),
    WithYear(Date<YearShown>),
}

impl DateMachine {
    #[inline]
    pub fn new(label: gtk::Label, epoch: i64) -> Self {
        let m = DateMachine::UnInitialized(Date::<UnInitialized>::new(label, epoch));
        m.determine_state(epoch)
    }

    #[inline]
    pub fn determine_state(self, epoch: i64) -> Self {
        use self::DateMachine::*;

        let ts = Utc.timestamp(epoch, 0);
        let is_old = !(NOW.year() == ts.year());

        match (self, is_old) {
            // Into Usual
            (Usual(val), false) => Usual(val.into_usual(epoch)),
            (WithYear(val), false) => Usual(val.into_usual(epoch)),
            (UnInitialized(val), false) => Usual(val.into_usual(epoch)),

            // Into Year Shown
            (Usual(val), true) => WithYear(val.into_year_shown(epoch)),
            (WithYear(val), true) => WithYear(val.into_year_shown(epoch)),
            (UnInitialized(val), true) => WithYear(val.into_year_shown(epoch)),
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
    #[inline]
    // This needs a better name.
    // TODO: make me mut
    fn set_duration(&self, minutes: i64) {
        self.duration.set_text(&format!("{} min", minutes));
    }
}

impl Duration<Hidden> {
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    pub fn new(duration: gtk::Label, separator: gtk::Label, seconds: Option<i32>) -> Self {
        let m = DurationMachine::Hidden(Duration::<Hidden>::new(duration, separator));
        m.determine_state(seconds)
    }

    #[inline]
    pub fn determine_state(self, seconds: Option<i32>) -> Self {
        match (self, seconds) {
            (d @ DurationMachine::Hidden(_), None) => d,
            (DurationMachine::Shown(val), None) => DurationMachine::Hidden(val.into()),
            (DurationMachine::Hidden(val), Some(s)) => {
                let minutes = chrono::Duration::seconds(s.into()).num_minutes();
                if minutes == 0 {
                    DurationMachine::Hidden(val)
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
                    DurationMachine::Shown(val)
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

impl<S> Size<S> {
    #[inline]
    fn set_size(self, s: &str) -> Size<Shown> {
        self.size.set_text(s);
        self.size.show();
        self.separator.show();
        Size {
            size: self.size,
            separator: self.separator,
            state: Shown {},
        }
    }

    #[inline]
    // https://play.rust-lang.org/?gist=1acffaf62743eeb85be1ae6ecf474784&version=stable
    // It might be possible to make a generic definition with Specialization.
    // https://github.com/rust-lang/rust/issues/31844
    fn into_shown(self) -> Size<Shown> {
        self.size.show();
        self.separator.show();

        Size {
            size: self.size,
            separator: self.separator,
            state: Shown {},
        }
    }

    #[inline]
    fn into_hidden(self) -> Size<Hidden> {
        self.size.hide();
        self.separator.hide();

        Size {
            size: self.size,
            separator: self.separator,
            state: Hidden {},
        }
    }
}

impl Size<UnInitialized> {
    #[inline]
    fn new(size: gtk::Label, separator: gtk::Label) -> Self {
        size.hide();
        separator.hide();

        Size {
            size,
            separator,
            state: UnInitialized {},
        }
    }
}

// pub trait Playable {}

// impl Playable for Download {}
// impl Playable for Play {}

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

impl<S> DownloadPlay<S> {
    #[inline]
    // https://play.rust-lang.org/?gist=1acffaf62743eeb85be1ae6ecf474784&version=stable
    // It might be possible to make a generic definition with Specialization.
    // https://github.com/rust-lang/rust/issues/31844
    fn into_playable(self) -> DownloadPlay<Play> {
        self.play.show();
        self.download.hide();

        DownloadPlay {
            play: self.play,
            download: self.download,
            state: Play {},
        }
    }

    #[inline]
    fn into_fetchable(self) -> DownloadPlay<Download> {
        self.play.hide();
        self.download.show();

        DownloadPlay {
            play: self.play,
            download: self.download,
            state: Download {},
        }
    }

    #[inline]
    fn into_hidden(self) -> DownloadPlay<Hidden> {
        self.play.hide();
        self.download.hide();

        DownloadPlay {
            play: self.play,
            download: self.download,
            state: Hidden {},
        }
    }

    #[inline]
    fn download_connect_clicked<F: Fn(&gtk::Button) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.download.connect_clicked(f)
    }

    #[inline]
    fn play_connect_clicked<F: Fn(&gtk::Button) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.play.connect_clicked(f)
    }
}

impl DownloadPlay<UnInitialized> {
    #[inline]
    fn new(play: gtk::Button, download: gtk::Button) -> Self {
        play.hide();
        download.hide();

        DownloadPlay {
            play,
            download,
            state: UnInitialized {},
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

impl<S> Progress<S> {
    #[inline]
    fn into_shown(self) -> Progress<Shown> {
        self.bar.show();
        self.cancel.show();
        self.local_size.show();
        self.prog_separator.show();

        Progress {
            bar: self.bar,
            cancel: self.cancel,
            local_size: self.local_size,
            prog_separator: self.prog_separator,
            state: Shown {},
        }
    }

    #[inline]
    fn into_hidden(self) -> Progress<Hidden> {
        self.bar.hide();
        self.cancel.hide();
        self.local_size.hide();
        self.prog_separator.hide();

        Progress {
            bar: self.bar,
            cancel: self.cancel,
            local_size: self.local_size,
            prog_separator: self.prog_separator,
            state: Hidden {},
        }
    }

    #[allow(unused_must_use)]
    #[inline]
    // This does not need to be &mut since gtk-rs does not model ownership
    // But I think it wouldn't hurt if we treat it as a Rust api.
    fn update_progress(&mut self, local_size: &str, fraction: f64) {
        self.local_size.set_text(local_size);
        self.bar.set_fraction(fraction);
    }

    #[inline]
    fn cancel_connect_clicked<F: Fn(&gtk::Button) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.cancel.connect_clicked(f)
    }
}

impl Progress<UnInitialized> {
    #[inline]
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
            state: UnInitialized {},
        }
    }
}

#[derive(Debug, Clone)]
pub struct Media<X, Y, Z> {
    dl: DownloadPlay<X>,
    size: Size<Y>,
    progress: Progress<Z>,
}

type New<Y> = Media<Download, Y, Hidden>;
type Playable<Y> = Media<Play, Y, Hidden>;
type InProgress = Media<Hidden, Shown, Shown>;

impl<X, Y, Z> Media<X, Y, Z> {
    #[inline]
    fn set_size(self, s: &str) -> Media<X, Shown, Z> {
        Media {
            dl: self.dl,
            size: self.size.set_size(s),
            progress: self.progress,
        }
    }

    #[inline]
    fn hide_size(self) -> Media<X, Hidden, Z> {
        Media {
            dl: self.dl,
            size: self.size.into_hidden(),
            progress: self.progress,
        }
    }

    #[inline]
    fn into_new(self, size: &str) -> New<Shown> {
        Media {
            dl: self.dl.into_fetchable(),
            size: self.size.set_size(size),
            progress: self.progress.into_hidden(),
        }
    }

    #[inline]
    fn into_new_without(self) -> New<Hidden> {
        Media {
            dl: self.dl.into_fetchable(),
            size: self.size.into_hidden(),
            progress: self.progress.into_hidden(),
        }
    }

    #[inline]
    fn into_playable(self, size: &str) -> Playable<Shown> {
        Media {
            dl: self.dl.into_playable(),
            size: self.size.set_size(size),
            progress: self.progress.into_hidden(),
        }
    }

    #[inline]
    fn into_playable_without(self) -> Playable<Hidden> {
        Media {
            dl: self.dl.into_playable(),
            size: self.size.into_hidden(),
            progress: self.progress.into_hidden(),
        }
    }
}

impl<X, Z> Media<X, Shown, Z> {
    #[inline]
    fn into_progress(self) -> InProgress {
        Media {
            dl: self.dl.into_hidden(),
            size: self.size.into_shown(),
            progress: self.progress.into_shown(),
        }
    }
}

impl<X, Z> Media<X, Hidden, Z> {
    #[inline]
    fn into_progress(self) -> InProgress {
        Media {
            dl: self.dl.into_hidden(),
            size: self.size.set_size("Unkown"),
            progress: self.progress.into_shown(),
        }
    }
}

impl<X, Z> Media<X, UnInitialized, Z> {
    #[inline]
    fn into_progress(self, size: Option<String>) -> InProgress {
        if let Some(s) = size {
            Media {
                dl: self.dl.into_hidden(),
                size: self.size.set_size(&s),
                progress: self.progress.into_shown(),
            }
        } else {
            Media {
                dl: self.dl.into_hidden(),
                size: self.size.set_size("Unkown"),
                progress: self.progress.into_shown(),
            }
        }
    }
}

impl InProgress {
    #[inline]
    #[allow(unused_must_use)]
    // This does not need to be &mut since gtk-rs does not model ownership
    // But I think it wouldn't hurt if we treat it as a Rust api.
    fn update_progress(&mut self, local_size: &str, fraction: f64) {
        self.progress.update_progress(local_size, fraction)
    }
}

#[derive(Debug, Clone)]
pub enum ButtonsState {
    New(Media<Download, Shown, Hidden>),
    NewWithoutSize(Media<Download, Hidden, Hidden>),
    Playable(Media<Play, Shown, Hidden>),
    PlayableWithoutSize(Media<Play, Hidden, Hidden>),
}

impl ButtonsState {
    #[inline]
    pub fn determine_state(self, size: Option<String>, is_downloaded: bool) -> Self {
        use self::ButtonsState::*;

        match (self, size, is_downloaded) {
            // From whatever to New
            (New(m), Some(s), false) => New(m.into_new(&s)),
            (Playable(m), Some(s), false) => New(m.into_new(&s)),

            (NewWithoutSize(m), Some(s), false) => New(m.into_new(&s)),
            (PlayableWithoutSize(m), Some(s), false) => New(m.into_new(&s)),

            // From whatever to Playable
            (New(m), Some(s), true) => Playable(m.into_playable(&s)),
            (Playable(m), Some(s), true) => Playable(m.into_playable(&s)),

            (NewWithoutSize(m), Some(s), true) => Playable(m.into_playable(&s)),
            (PlayableWithoutSize(m), Some(s), true) => Playable(m.into_playable(&s)),

            // From whatever to NewWithoutSize
            (New(m), None, false) => NewWithoutSize(m.hide_size()),
            (Playable(m), None, false) => NewWithoutSize(m.into_new_without()),

            (b @ NewWithoutSize(_), None, false) => b,
            (PlayableWithoutSize(m), None, false) => NewWithoutSize(m.into_new_without()),

            // From whatever to PlayableWithoutSize
            (New(m), None, true) => PlayableWithoutSize(m.into_playable_without()),
            (Playable(m), None, true) => PlayableWithoutSize(m.hide_size()),

            (NewWithoutSize(val), None, true) => PlayableWithoutSize(val.into_playable_without()),
            (b @ PlayableWithoutSize(_), None, true) => b,
        }
    }

    #[inline]
    fn into_progress(self) -> InProgress {
        use self::ButtonsState::*;

        match self {
            New(m) => m.into_progress(),
            Playable(m) => m.into_progress(),
            NewWithoutSize(m) => m.into_progress(),
            PlayableWithoutSize(m) => m.into_progress(),
        }
    }

    #[inline]
    fn set_size(self, size: Option<String>) -> Self {
        use self::ButtonsState::*;

        match (self, size) {
            (New(m), Some(s)) => New(m.set_size(&s)),
            (New(m), None) => NewWithoutSize(m.hide_size()),
            (Playable(m), Some(s)) => Playable(m.set_size(&s)),
            (Playable(m), None) => PlayableWithoutSize(m.hide_size()),
            (bttn @ NewWithoutSize(_), None) => bttn,
            (bttn @ PlayableWithoutSize(_), None) => bttn,
            (NewWithoutSize(m), Some(s)) => New(m.into_new(&s)),
            (PlayableWithoutSize(m), Some(s)) => Playable(m.into_playable(&s)),
        }
    }

    #[inline]
    pub fn download_connect_clicked<F: Fn(&gtk::Button) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        use self::ButtonsState::*;

        match *self {
            New(ref val) => val.dl.download_connect_clicked(f),
            NewWithoutSize(ref val) => val.dl.download_connect_clicked(f),
            Playable(ref val) => val.dl.download_connect_clicked(f),
            PlayableWithoutSize(ref val) => val.dl.download_connect_clicked(f),
        }
    }

    #[inline]
    pub fn play_connect_clicked<F: Fn(&gtk::Button) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        use self::ButtonsState::*;

        match *self {
            New(ref val) => val.dl.play_connect_clicked(f),
            NewWithoutSize(ref val) => val.dl.play_connect_clicked(f),
            Playable(ref val) => val.dl.play_connect_clicked(f),
            PlayableWithoutSize(ref val) => val.dl.play_connect_clicked(f),
        }
    }

    #[inline]
    fn cancel_connect_clicked<F: Fn(&gtk::Button) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        use self::ButtonsState::*;

        match *self {
            New(ref val) => val.progress.cancel_connect_clicked(f),
            NewWithoutSize(ref val) => val.progress.cancel_connect_clicked(f),
            Playable(ref val) => val.progress.cancel_connect_clicked(f),
            PlayableWithoutSize(ref val) => val.progress.cancel_connect_clicked(f),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MediaMachine {
    UnInitialized(Media<UnInitialized, UnInitialized, UnInitialized>),
    Initialized(ButtonsState),
    InProgress(Media<Hidden, Shown, Shown>),
}

impl MediaMachine {
    #[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
    #[inline]
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
        let dl = DownloadPlay::<UnInitialized>::new(play, download);
        let progress = Progress::<UnInitialized>::new(bar, cancel, local_size, prog_separator);
        let size = Size::<UnInitialized>::new(total_size, separator);

        MediaMachine::UnInitialized(Media { dl, progress, size })
    }

    #[inline]
    pub fn download_connect_clicked<F: Fn(&gtk::Button) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        use self::MediaMachine::*;

        match *self {
            UnInitialized(ref val) => val.dl.download_connect_clicked(f),
            Initialized(ref val) => val.download_connect_clicked(f),
            InProgress(ref val) => val.dl.download_connect_clicked(f),
        }
    }

    #[inline]
    pub fn play_connect_clicked<F: Fn(&gtk::Button) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        use self::MediaMachine::*;

        match *self {
            UnInitialized(ref val) => val.dl.play_connect_clicked(f),
            Initialized(ref val) => val.play_connect_clicked(f),
            InProgress(ref val) => val.dl.play_connect_clicked(f),
        }
    }

    #[inline]
    pub fn cancel_connect_clicked<F: Fn(&gtk::Button) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        use self::MediaMachine::*;

        match *self {
            UnInitialized(ref val) => val.progress.cancel_connect_clicked(f),
            Initialized(ref val) => val.cancel_connect_clicked(f),
            InProgress(ref val) => val.progress.cancel_connect_clicked(f),
        }
    }

    #[inline]
    pub fn determine_state(self, bytes: Option<i32>, is_active: bool, is_downloaded: bool) -> Self {
        use self::ButtonsState::*;
        use self::MediaMachine::*;

        match (self, size_helper(bytes), is_downloaded, is_active) {
            (UnInitialized(m), s, _, true) => InProgress(m.into_progress(s)),

            // Into New
            (UnInitialized(m), Some(s), false, false) => Initialized(New(m.into_new(&s))),
            (UnInitialized(m), None, false, false) => {
                Initialized(NewWithoutSize(m.into_new_without()))
            }

            // Into Playable
            (UnInitialized(m), Some(s), true, false) => Initialized(Playable(m.into_playable(&s))),
            (UnInitialized(m), None, true, false) => {
                Initialized(PlayableWithoutSize(m.into_playable_without()))
            }

            (Initialized(bttn), s, dl, false) => Initialized(bttn.determine_state(s, dl)),
            (Initialized(bttn), _, _, true) => InProgress(bttn.into_progress()),

            // Into New
            (InProgress(m), Some(s), false, false) => Initialized(New(m.into_new(&s))),
            (InProgress(m), None, false, false) => {
                Initialized(NewWithoutSize(m.into_new_without()))
            }

            // Into Playable
            (InProgress(m), Some(s), true, false) => Initialized(Playable(m.into_playable(&s))),
            (InProgress(m), None, true, false) => {
                Initialized(PlayableWithoutSize(m.into_playable_without()))
            }

            (i @ InProgress(_), _, _, _) => i,
        }
    }

    #[inline]
    pub fn set_size(self, bytes: Option<i32>) -> Self {
        use self::MediaMachine::*;
        let size = size_helper(bytes);

        match (self, size) {
            (Initialized(bttn), s) => Initialized(bttn.set_size(s)),
            (InProgress(val), Some(s)) => InProgress(val.set_size(&s)),
            (n @ InProgress(_), None) => n,
            (n @ UnInitialized(_), _) => n,
        }
    }

    #[inline]
    pub fn update_progress(&mut self, local_size: &str, fraction: f64) {
        use self::MediaMachine::*;

        match *self {
            Initialized(_) => (),
            UnInitialized(_) => (),
            InProgress(ref mut val) => val.update_progress(local_size, fraction),
        }
    }
}

#[inline]
fn size_helper(bytes: Option<i32>) -> Option<String> {
    let s = bytes?;
    if s == 0 {
        return None;
    }

    s.file_size(SIZE_OPTS.clone()).ok()
}

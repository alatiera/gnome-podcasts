//  TODO: Things that should be done.
//
// * Wherever there's a function that take 2 or more arguments of the same type,
//   eg: fn new(total_size: gtk::Label, local_size: gtk::Label ..)
//   Wrap the types into Struct-tuples and imple deref so it won't be possible to pass
//   the wrong argument to the wrong position.

use chrono;
use glib;
use gtk;
use gtk::prelude::*;

use std::sync::{Arc, Mutex};

use manager::Progress as OtherProgress;
use widgets::episode::SIZE_OPTS;

#[derive(Debug, Clone)]
pub struct UnInitialized;

#[derive(Debug, Clone)]
pub struct Shown;
#[derive(Debug, Clone)]
pub struct Hidden;

pub trait Visibility {}

impl Visibility for Shown {}
impl Visibility for Hidden {}

impl From<Hidden> for Shown {
    fn from(_: Hidden) -> Self {
        Shown {}
    }
}

impl From<Shown> for Hidden {
    fn from(_: Shown) -> Self {
        Hidden {}
    }
}

impl Into<Hidden> for UnInitialized {
    fn into(self) -> Hidden {
        Hidden {}
    }
}

impl Into<Shown> for UnInitialized {
    fn into(self) -> Shown {
        Shown {}
    }
}

#[derive(Debug, Clone)]
pub struct Normal;
#[derive(Debug, Clone)]
pub struct GreyedOut;

impl From<Normal> for GreyedOut {
    fn from(_: Normal) -> Self {
        GreyedOut {}
    }
}

impl From<GreyedOut> for Normal {
    fn from(_: GreyedOut) -> Self {
        Normal {}
    }
}

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
    fn from(f: Title<Normal>) -> Self {
        f.title
            .get_style_context()
            .map(|c| c.add_class("dim-label"));

        Title {
            title: f.title,
            state: f.state.into(),
        }
    }
}

impl From<Title<GreyedOut>> for Title<Normal> {
    fn from(f: Title<GreyedOut>) -> Self {
        f.title
            .get_style_context()
            .map(|c| c.remove_class("dim-label"));

        Title {
            title: f.title,
            state: f.state.into(),
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
        use self::TitleMachine::*;

        match (self, is_played) {
            (title @ Normal(_), false) => title,
            (title @ GreyedOut(_), true) => title,
            (Normal(val), true) => GreyedOut(val.into()),
            (GreyedOut(val), false) => Normal(val.into()),
        }
    }

    pub fn set_title(&mut self, s: &str) {
        use self::TitleMachine::*;

        match *self {
            Normal(ref mut val) => val.set_title(s),
            GreyedOut(ref mut val) => val.set_title(s),
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
            state: f.state.into(),
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
            state: f.state.into(),
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

impl<S> Size<S> {
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

    fn into_fetchable(self) -> DownloadPlay<Download> {
        self.play.hide();
        self.download.show();

        DownloadPlay {
            play: self.play,
            download: self.download,
            state: Download {},
        }
    }

    fn into_hidden(self) -> DownloadPlay<Hidden> {
        self.play.hide();
        self.download.hide();

        DownloadPlay {
            play: self.play,
            download: self.download,
            state: Hidden {},
        }
    }

    fn download_connect_clicked<F: Fn(&gtk::Button) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.download.connect_clicked(f)
    }

    fn play_connect_clicked<F: Fn(&gtk::Button) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.play.connect_clicked(f)
    }
}

impl DownloadPlay<UnInitialized> {
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

    fn cancel_connect_clicked(&self, prog: Arc<Mutex<OtherProgress>>) -> glib::SignalHandlerId {
        self.cancel.connect_clicked(move |cancel| {
            if let Ok(mut m) = prog.lock() {
                m.cancel();
                cancel.set_sensitive(false);
            }
        })
    }
}

impl Progress<UnInitialized> {
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
type MediaUnInitialized = Media<UnInitialized, UnInitialized, UnInitialized>;

impl From<New<Shown>> for InProgress {
    fn from(f: New<Shown>) -> Self {
        f.into_progress()
    }
}

impl From<New<Hidden>> for InProgress {
    fn from(f: New<Hidden>) -> Self {
        f.into_progress()
    }
}

impl From<Playable<Shown>> for InProgress {
    fn from(f: Playable<Shown>) -> Self {
        f.into_progress()
    }
}

impl From<Playable<Hidden>> for InProgress {
    fn from(f: Playable<Hidden>) -> Self {
        f.into_progress()
    }
}

impl<Y: Visibility> From<Playable<Y>> for New<Y> {
    fn from(f: Playable<Y>) -> Self {
        Media {
            dl: f.dl.into_fetchable(),
            size: f.size,
            progress: f.progress,
        }
    }
}

impl<Y: Visibility> From<New<Y>> for Playable<Y> {
    fn from(f: New<Y>) -> Self {
        Media {
            dl: f.dl.into_playable(),
            size: f.size,
            progress: f.progress,
        }
    }
}

impl From<MediaUnInitialized> for New<Hidden> {
    fn from(f: MediaUnInitialized) -> Self {
        Media {
            dl: f.dl.into_fetchable(),
            size: f.size.into_hidden(),
            progress: f.progress.into_hidden(),
        }
    }
}

impl From<MediaUnInitialized> for Playable<Hidden> {
    fn from(f: MediaUnInitialized) -> Self {
        Media {
            dl: f.dl.into_playable(),
            size: f.size.into_hidden(),
            progress: f.progress.into_hidden(),
        }
    }
}

impl<X, Y, Z> Media<X, Y, Z> {
    // fn set_size(self, s: &str) -> Media<X, Shown, Z> {
    //     Media {
    //         dl: self.dl,
    //         size: self.size.set_size(s),
    //         progress: self.progress,
    //     }
    // }

    fn hide_size(self) -> Media<X, Hidden, Z> {
        Media {
            dl: self.dl,
            size: self.size.into_hidden(),
            progress: self.progress,
        }
    }

    fn into_new(self, size: &str) -> New<Shown> {
        Media {
            dl: self.dl.into_fetchable(),
            size: self.size.set_size(size),
            progress: self.progress.into_hidden(),
        }
    }

    fn into_playable(self, size: &str) -> Playable<Shown> {
        Media {
            dl: self.dl.into_playable(),
            size: self.size.set_size(size),
            progress: self.progress.into_hidden(),
        }
    }
}

impl<X, Z> Media<X, Shown, Z> {
    fn into_progress(self) -> InProgress {
        Media {
            dl: self.dl.into_hidden(),
            size: self.size.into_shown(),
            progress: self.progress.into_shown(),
        }
    }
}

impl<X, Z> Media<X, Hidden, Z> {
    fn into_progress(self) -> InProgress {
        Media {
            dl: self.dl.into_hidden(),
            size: self.size.set_size("Unkown"),
            progress: self.progress.into_shown(),
        }
    }
}

impl<X, Z> Media<X, UnInitialized, Z> {
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

#[derive(Debug, Clone)]
pub enum ButtonsState {
    New(Media<Download, Shown, Hidden>),
    NewWithoutSize(Media<Download, Hidden, Hidden>),
    Playable(Media<Play, Shown, Hidden>),
    PlayableWithoutSize(Media<Play, Hidden, Hidden>),
}

impl ButtonsState {
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
            (Playable(m), None, false) => NewWithoutSize(Media::from(m).hide_size()),

            (b @ NewWithoutSize(_), None, false) => b,
            (PlayableWithoutSize(m), None, false) => NewWithoutSize(m.into()),

            // From whatever to PlayableWithoutSize
            (New(m), None, true) => PlayableWithoutSize(Media::from(m).hide_size()),
            (Playable(m), None, true) => PlayableWithoutSize(Media::from(m).hide_size()),

            (NewWithoutSize(val), None, true) => PlayableWithoutSize(val.into()),
            (b @ PlayableWithoutSize(_), None, true) => b,
            // _ => unimplemented!()
        }
    }

    pub fn into_progress(self) -> InProgress {
        use self::ButtonsState::*;

        match self {
            New(m) => m.into(),
            Playable(m) => m.into(),
            NewWithoutSize(m) => m.into(),
            PlayableWithoutSize(m) => m.into(),
        }
    }

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

    fn cancel_connect_clicked(&self, prog: Arc<Mutex<OtherProgress>>) -> glib::SignalHandlerId {
        use self::ButtonsState::*;

        match *self {
            New(ref val) => val.progress.cancel_connect_clicked(prog),
            NewWithoutSize(ref val) => val.progress.cancel_connect_clicked(prog),
            Playable(ref val) => val.progress.cancel_connect_clicked(prog),
            PlayableWithoutSize(ref val) => val.progress.cancel_connect_clicked(prog),
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

    pub fn cancel_connect_clicked(&self, prog: Arc<Mutex<OtherProgress>>) -> glib::SignalHandlerId {
        use self::MediaMachine::*;

        match *self {
            UnInitialized(ref val) => val.progress.cancel_connect_clicked(prog),
            Initialized(ref val) => val.cancel_connect_clicked(prog),
            InProgress(ref val) => val.progress.cancel_connect_clicked(prog),
        }
    }

    pub fn determine_state(self, bytes: Option<i32>, is_active: bool, is_downloaded: bool) -> Self {
        use self::ButtonsState::*;
        use self::MediaMachine::*;
        use humansize::FileSize;

        let size_helper = || -> Option<String> {
            let s = bytes?;
            if s == 0 {
                return None;
            }

            s.file_size(SIZE_OPTS.clone()).ok()
        };

        match (self, size_helper(), is_downloaded, is_active) {
            (UnInitialized(m), s, _, true) => InProgress(m.into_progress(s)),

            // Into New
            (UnInitialized(m), Some(s), false, false) => Initialized(New(m.into_new(&s))),
            (UnInitialized(m), None, false, false) => Initialized(NewWithoutSize(m.into())),

            // Into Playable
            (UnInitialized(m), Some(s), true, false) => Initialized(Playable(m.into_playable(&s))),
            (UnInitialized(m), None, true, false) => Initialized(PlayableWithoutSize(m.into())),

            (Initialized(bttn), s, dl, false) => Initialized(bttn.determine_state(s, dl)),
            (Initialized(bttn), _, _, true) => InProgress(bttn.into_progress()),
            (i @ InProgress(_), _, _, _) => i,
        }
    }
}

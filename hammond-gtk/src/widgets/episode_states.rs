//  TODO: Things that should be done.
//
// * Wherever there's a function that take 2 or more arguments of the same type,
//   eg: fn new(total_size: gtk::Label, local_size: gtk::Label ..)
//   Wrap the types into Struct-tuples and imple deref so it won't be possible to pass
//   the wrong argument to the wrong position.

use gtk;

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
}

#[derive(Debug, Clone)]
pub struct Shown;
#[derive(Debug, Clone)]
pub struct Hidden;

pub trait Visibility {}

impl Visibility for Shown {}
impl Visibility for Hidden {}

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
    // https://play.rust-lang.org/?gist=1acffaf62743eeb85be1ae6ecf474784&version=stable // It might be possible to make a generic definition with Specialization.
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

    #[allow(unused_must_use)]
    // This does not need to be &mut since gtk-rs does not model ownership
    // But I think it wouldn't hurt if we treat it as a Rust api.
    fn update_progress(&mut self, local_size: &str, fraction: f64) {
        self.local_size.set_text(local_size);
        self.bar.set_fraction(fraction);
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
    fn set_size(self, s: &str) -> Media<X, Shown, Z> {
        Media {
            dl: self.dl,
            size: self.size.set_size(s),
            progress: self.progress,
        }
    }

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

    fn into_new_without(self) -> New<Hidden> {
        Media {
            dl: self.dl.into_fetchable(),
            size: self.size.into_hidden(),
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

    fn into_playable_without(self) -> Playable<Hidden> {
        Media {
            dl: self.dl.into_playable(),
            size: self.size.into_hidden(),
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

impl InProgress {
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

    fn into_progress(self) -> InProgress {
        use self::ButtonsState::*;

        match self {
            New(m) => m.into_progress(),
            Playable(m) => m.into_progress(),
            NewWithoutSize(m) => m.into_progress(),
            PlayableWithoutSize(m) => m.into_progress(),
        }
    }

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
}

#[derive(Debug, Clone)]
pub enum MediaMachine {
    Initialized(ButtonsState),
    InProgress(Media<Hidden, Shown, Shown>),
}

impl MediaMachine {
    pub fn determine_state(self, bytes: Option<i32>, is_active: bool, is_downloaded: bool) -> Self {
        use self::ButtonsState::*;
        use self::MediaMachine::*;

        match (self, size_helper(bytes), is_downloaded, is_active) {
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

    pub fn set_size(self, bytes: Option<i32>) -> Self {
        use self::MediaMachine::*;
        let size = size_helper(bytes);

        match (self, size) {
            (Initialized(bttn), s) => Initialized(bttn.set_size(s)),
            (InProgress(val), Some(s)) => InProgress(val.set_size(&s)),
            (n @ InProgress(_), None) => n,
        }
    }

    pub fn update_progress(&mut self, local_size: &str, fraction: f64) {
        use self::MediaMachine::*;

        match *self {
            Initialized(_) => (),
            InProgress(ref mut val) => val.update_progress(local_size, fraction),
        }
    }
}

fn size_helper(bytes: Option<i32>) -> Option<String> {
    let s = bytes?;
    if s == 0 {
        return None;
    }

    s.file_size(SIZE_OPTS.clone()).ok()
}

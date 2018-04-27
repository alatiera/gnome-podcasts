use glib;
use gtk;
use gtk::prelude::*;

use failure::Error;
use html2pango::markup_from_raw;
use open;
use rayon;
use send_cell::SendCell;

use hammond_data::dbqueries;
use hammond_data::utils::delete_show;
use hammond_data::Podcast;

use app::Action;
use appnotif::InAppNotification;
use utils::{self, lazy_load};
use widgets::EpisodeWidget;

use std::rc::Rc;
use std::sync::mpsc::{SendError, Sender};
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref SHOW_WIDGET_VALIGNMENT: Mutex<Option<(i32, SendCell<gtk::Adjustment>)>> =
        Mutex::new(None);
}

#[derive(Debug, Clone)]
pub struct ShowWidget {
    pub container: gtk::Box,
    scrolled_window: gtk::ScrolledWindow,
    cover: gtk::Image,
    description: gtk::Label,
    link: gtk::Button,
    settings: gtk::MenuButton,
    unsub: gtk::Button,
    episodes: gtk::ListBox,
    podcast_id: Option<i32>,
}

impl Default for ShowWidget {
    #[inline]
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/show_widget.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let scrolled_window: gtk::ScrolledWindow = builder.get_object("scrolled_window").unwrap();
        let episodes = builder.get_object("episodes").unwrap();

        let cover: gtk::Image = builder.get_object("cover").unwrap();
        let description: gtk::Label = builder.get_object("description").unwrap();
        let unsub: gtk::Button = builder.get_object("unsub_button").unwrap();
        let link: gtk::Button = builder.get_object("link_button").unwrap();
        let settings: gtk::MenuButton = builder.get_object("settings_button").unwrap();

        ShowWidget {
            container,
            scrolled_window,
            cover,
            description,
            unsub,
            link,
            settings,
            episodes,
            podcast_id: None,
        }
    }
}

impl ShowWidget {
    #[inline]
    pub fn new(pd: Arc<Podcast>, sender: Sender<Action>) -> Rc<ShowWidget> {
        let mut pdw = ShowWidget::default();
        pdw.init(pd.clone(), sender.clone());
        let pdw = Rc::new(pdw);
        populate_listbox(&pdw, pd, sender)
            .map_err(|err| error!("Failed to populate the listbox: {}", err))
            .ok();

        pdw
    }

    #[inline]
    pub fn init(&mut self, pd: Arc<Podcast>, sender: Sender<Action>) {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/show_widget.ui");

        self.unsub
            .connect_clicked(clone!(pd, sender => move |bttn| {
                on_unsub_button_clicked(pd.clone(), bttn, sender.clone());
        }));

        self.set_description(pd.description());
        self.podcast_id = Some(pd.id());

        self.set_cover(pd.clone())
            .map_err(|err| error!("Failed to set a cover: {}", err))
            .ok();

        let link = pd.link().to_owned();
        self.link.set_tooltip_text(Some(link.as_str()));
        self.link.connect_clicked(move |_| {
            info!("Opening link: {}", &link);
            open::that(&link)
                .map_err(|err| error!("Error: {}", err))
                .map_err(|_| error!("Failed open link: {}", &link))
                .ok();
        });

        let show_menu: gtk::Popover = builder.get_object("show_menu").unwrap();
        let mark_all: gtk::ModelButton = builder.get_object("mark_all_watched").unwrap();

        let episodes = self.episodes.clone();
        mark_all.connect_clicked(clone!(pd, sender => move |_| {
            on_played_button_clicked(
                pd.clone(),
                &episodes,
                sender.clone()
            )
        }));
        self.settings.set_popover(&show_menu);
    }

    #[inline]
    /// Set the show cover.
    fn set_cover(&self, pd: Arc<Podcast>) -> Result<(), Error> {
        utils::set_image_from_path(&self.cover, pd.id(), 256)
    }

    #[inline]
    /// Set the descripton text.
    fn set_description(&self, text: &str) {
        self.description.set_markup(&markup_from_raw(text));
    }

    #[inline]
    /// Save the scrollabar vajustment to the cache.
    pub fn save_vadjustment(&self, oldid: i32) -> Result<(), Error> {
        if let Ok(mut guard) = SHOW_WIDGET_VALIGNMENT.lock() {
            let adj = self.scrolled_window
                .get_vadjustment()
                .ok_or_else(|| format_err!("Could not get the adjustment"))?;
            *guard = Some((oldid, SendCell::new(adj)));
            debug!("Widget Alignment was saved with ID: {}.", oldid);
        }

        Ok(())
    }

    #[inline]
    /// Set scrolled window vertical adjustment.
    fn set_vadjustment(&self, pd: Arc<Podcast>) -> Result<(), Error> {
        let guard = SHOW_WIDGET_VALIGNMENT
            .lock()
            .map_err(|err| format_err!("Failed to lock widget align mutex: {}", err))?;

        if let Some((oldid, ref sendcell)) = *guard {
            // Only copy the old scrollbar if both widget's represent the same podcast.
            debug!("PID: {}", pd.id());
            debug!("OLDID: {}", oldid);
            if pd.id() != oldid {
                debug!("Early return");
                return Ok(());
            };

            // Copy the vertical scrollbar adjustment from the old view into the new one.
            sendcell
                .try_get()
                .map(|x| self.scrolled_window.set_vadjustment(&x));
        }

        Ok(())
    }

    pub fn podcast_id(&self) -> Option<i32> {
        self.podcast_id
    }
}

#[inline]
/// Populate the listbox with the shows episodes.
fn populate_listbox(
    show: &Rc<ShowWidget>,
    pd: Arc<Podcast>,
    sender: Sender<Action>,
) -> Result<(), Error> {
    use crossbeam_channel::bounded;
    use crossbeam_channel::TryRecvError::*;

    let count = dbqueries::get_pd_episodes_count(&pd)?;

    let (sender_, receiver) = bounded(1);
    rayon::spawn(clone!(pd => move || {
        let episodes = dbqueries::get_pd_episodeswidgets(&pd).unwrap();
        // The receiver can be dropped if there's an early return
        // like on show without episodes for example.
        sender_.send(episodes).ok();
    }));

    if count == 0 {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/empty_show.ui");
        let container: gtk::Box = builder.get_object("empty_show").unwrap();
        show.episodes.add(&container);
        return Ok(());
    }

    let show_ = show.clone();
    gtk::idle_add(move || {
        let episodes = match receiver.try_recv() {
            Ok(e) => e,
            Err(Empty) => return glib::Continue(true),
            Err(Disconnected) => return glib::Continue(false),
        };

        let list = show_.episodes.clone();

        let constructor = clone!(sender => move |ep| {
            EpisodeWidget::new(ep, sender.clone()).container
        });

        let callback = clone!(pd, show_ => move || {
            show_.set_vadjustment(pd.clone())
                .map_err(|err| error!("Failed to set ShowWidget Alignment: {}", err))
                .ok();
        });

        lazy_load(episodes, list.clone(), constructor, callback);

        glib::Continue(false)
    });

    Ok(())
}

#[inline]
fn on_unsub_button_clicked(pd: Arc<Podcast>, unsub_button: &gtk::Button, sender: Sender<Action>) {
    // hack to get away without properly checking for none.
    // if pressed twice would panic.
    unsub_button.set_sensitive(false);

    let wrap = || -> Result<(), SendError<_>> {
        sender.send(Action::RemoveShow(pd))?;

        sender.send(Action::HeaderBarNormal)?;
        sender.send(Action::ShowShowsAnimated)?;
        // Queue a refresh after the switch to avoid blocking the db.
        sender.send(Action::RefreshShowsView)?;
        sender.send(Action::RefreshEpisodesView)?;
        Ok(())
    };

    wrap().map_err(|err| error!("Action Sender: {}", err)).ok();
    unsub_button.set_sensitive(true);
}

#[inline]
fn on_played_button_clicked(pd: Arc<Podcast>, episodes: &gtk::ListBox, sender: Sender<Action>) {
    if dim_titles(episodes).is_none() {
        error!("Something went horribly wrong when dimming the titles.");
        warn!("RUN WHILE YOU STILL CAN!");
    }

    sender
        .send(Action::MarkAllPlayerNotification(pd))
        .map_err(|err| error!("Action Sender: {}", err))
        .ok();
}

#[inline]
fn mark_all_watched(pd: &Podcast, sender: Sender<Action>) -> Result<(), Error> {
    dbqueries::update_none_to_played_now(pd)?;
    // Not all widgets migth have been loaded when the mark_all is hit
    // So we will need to refresh again after it's done.
    sender.send(Action::RefreshWidgetIfSame(pd.id()))?;
    sender.send(Action::RefreshEpisodesView).map_err(From::from)
}

#[inline]
pub fn mark_all_notif(pd: Arc<Podcast>, sender: Sender<Action>) -> InAppNotification {
    let id = pd.id();
    let callback = clone!(sender => move || {
        mark_all_watched(&pd, sender.clone())
            .map_err(|err| error!("Notif Callback Error: {}", err))
            .ok();
        glib::Continue(false)
    });

    let undo_callback = clone!(sender => move || {
        sender.send(Action::RefreshWidgetIfSame(id))
            .map_err(|err| error!("Action Sender: {}", err))
            .ok();
    });

    let text = "Marked all episodes as listened".into();
    InAppNotification::new(text, callback, undo_callback)
}

#[inline]
pub fn remove_show_notif(pd: Arc<Podcast>, sender: Sender<Action>) -> InAppNotification {
    let text = format!("Unsubscribed from {}", pd.title());

    utils::ignore_show(pd.id())
        .map_err(|err| error!("Error: {}", err))
        .map_err(|_| error!("Could not insert {} to the ignore list.", pd.title()))
        .ok();

    let callback = clone!(pd, sender => move || {
        utils::uningore_show(pd.id())
            .map_err(|err| error!("Error: {}", err))
            .map_err(|_| error!("Could not remove {} from the ignore list.", pd.title()))
            .ok();

        // Spawn a thread so it won't block the ui.
        rayon::spawn(clone!(pd, sender => move || {
            delete_show(&pd)
                .map_err(|err| error!("Error: {}", err))
                .map_err(|_| error!("Failed to delete {}", pd.title()))
                .ok();

            sender.send(Action::RefreshEpisodesView).ok();
        }));
        glib::Continue(false)
    });

    let undo_wrap = move || -> Result<(), Error> {
        utils::uningore_show(pd.id())?;
        sender.send(Action::RefreshShowsView)?;
        sender.send(Action::RefreshEpisodesView)?;
        Ok(())
    };

    let undo_callback = move || {
        undo_wrap().map_err(|err| error!("{}", err)).ok();
    };

    InAppNotification::new(text, callback, undo_callback)
}

#[inline]
// Ideally if we had a custom widget this would have been as simple as:
// `for row in listbox { ep = row.get_episode(); ep.dim_title(); }`
// But now I can't think of a better way to do it than hardcoding the title
// position relative to the EpisodeWidget container gtk::Box.
fn dim_titles(episodes: &gtk::ListBox) -> Option<()> {
    let children = episodes.get_children();

    for row in children {
        let row = row.downcast::<gtk::ListBoxRow>().ok()?;
        let container = row.get_children().remove(0).downcast::<gtk::Box>().ok()?;
        let foo = container
            .get_children()
            .remove(0)
            .downcast::<gtk::Box>()
            .ok()?;
        let bar = foo.get_children().remove(0).downcast::<gtk::Box>().ok()?;
        let baz = bar.get_children().remove(0).downcast::<gtk::Box>().ok()?;
        let title = baz.get_children().remove(0).downcast::<gtk::Label>().ok()?;

        title.get_style_context().map(|c| c.add_class("dim-label"));
    }
    Some(())
}

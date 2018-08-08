use glib;
use gtk;
use gtk::prelude::*;

use crossbeam_channel::Sender;
use failure::Error;
use fragile::Fragile;
use html2text;
use rayon;

use podcasts_data::dbqueries;
use podcasts_data::Show;

use app::Action;
use utils::{self, lazy_load};
use widgets::{EpisodeWidget, ShowMenu};

use std::rc::Rc;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref SHOW_WIDGET_VALIGNMENT: Mutex<Option<(i32, Fragile<gtk::Adjustment>)>> =
        Mutex::new(None);
}

#[derive(Debug, Clone)]
pub(crate) struct ShowWidget {
    pub(crate) container: gtk::Box,
    scrolled_window: gtk::ScrolledWindow,
    cover: gtk::Image,
    description: gtk::Label,
    frame: gtk::Frame,
    episodes: gtk::ListBox,
    show_id: Option<i32>,
}

impl Default for ShowWidget {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/show_widget.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let scrolled_window: gtk::ScrolledWindow = builder.get_object("scrolled_window").unwrap();
        let frame: gtk::Frame = builder.get_object("frame").unwrap();
        let episodes = gtk::ListBox::new();
        episodes.show();
        frame.add(&episodes);

        let cover: gtk::Image = builder.get_object("cover").unwrap();
        let description: gtk::Label = builder.get_object("description").unwrap();

        ShowWidget {
            container,
            scrolled_window,
            cover,
            description,
            frame,
            episodes,
            show_id: None,
        }
    }
}

impl ShowWidget {
    pub(crate) fn new(pd: Arc<Show>, sender: Sender<Action>) -> Rc<ShowWidget> {
        let mut pdw = ShowWidget::default();
        pdw.init(&pd);

        let menu = ShowMenu::new(&pd, &pdw.episodes, &sender);
        sender.send(Action::InitShowMenu(Fragile::new(menu)));

        let pdw = Rc::new(pdw);
        let res = populate_listbox(&pdw, pd.clone(), sender);
        debug_assert!(res.is_ok());

        pdw
    }

    pub(crate) fn init(&mut self, pd: &Arc<Show>) {
        self.set_description(pd.description());
        self.show_id = Some(pd.id());

        let res = self.set_cover(&pd);
        debug_assert!(res.is_ok());
    }

    /// Set the show cover.
    fn set_cover(&self, pd: &Arc<Show>) -> Result<(), Error> {
        utils::set_image_from_path(&self.cover, pd.id(), 256)
    }

    /// Set the description text.
    fn set_description(&self, text: &str) {
        self.description
            .set_markup(html2text::from_read(text.as_bytes(), 70).trim());
    }

    /// Save the scrollbar adjustment to the cache.
    pub(crate) fn save_vadjustment(&self, oldid: i32) -> Result<(), Error> {
        if let Ok(mut guard) = SHOW_WIDGET_VALIGNMENT.lock() {
            let adj = self
                .scrolled_window
                .get_vadjustment()
                .ok_or_else(|| format_err!("Could not get the adjustment"))?;
            *guard = Some((oldid, Fragile::new(adj)));
            debug!("Widget Alignment was saved with ID: {}.", oldid);
        }

        Ok(())
    }

    /// Set scrolled window vertical adjustment.
    fn set_vadjustment(&self, pd: &Arc<Show>) -> Result<(), Error> {
        let guard = SHOW_WIDGET_VALIGNMENT
            .lock()
            .map_err(|err| format_err!("Failed to lock widget align mutex: {}", err))?;

        if let Some((oldid, ref fragile)) = *guard {
            // Only copy the old scrollbar if both widgets represent the same podcast.
            debug!("PID: {}", pd.id());
            debug!("OLDID: {}", oldid);
            if pd.id() != oldid {
                debug!("Early return");
                return Ok(());
            };

            // Copy the vertical scrollbar adjustment from the old view into the new one.
            let res = fragile
                .try_get()
                .map(|x| utils::smooth_scroll_to(&self.scrolled_window, &x))
                .map_err(From::from);

            debug_assert!(res.is_ok());
            return res;
        }

        Ok(())
    }

    pub(crate) fn show_id(&self) -> Option<i32> {
        self.show_id
    }
}

/// Populate the listbox with the shows episodes.
fn populate_listbox(
    show: &Rc<ShowWidget>,
    pd: Arc<Show>,
    sender: Sender<Action>,
) -> Result<(), Error> {
    use crossbeam_channel::bounded;

    let count = dbqueries::get_pd_episodes_count(&pd)?;

    let (sender_, receiver) = bounded(1);
    rayon::spawn(clone!(pd => move || {
        if let Ok(episodes) = dbqueries::get_pd_episodeswidgets(&pd) {
            // The receiver can be dropped if there's an early return
            // like on show without episodes for example.
            sender_.send(episodes);
        }
    }));

    if count == 0 {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/empty_show.ui");
        let container: gtk::Box = builder
            .get_object("empty_show")
            .ok_or_else(|| format_err!("FOO"))?;
        show.episodes.add(&container);
        return Ok(());
    }

    let show_ = show.clone();
    gtk::idle_add(move || {
        let episodes = match receiver.try_recv() {
            Some(e) => e,
            None => return glib::Continue(true),
        };
        debug_assert!(episodes.len() as i64 == count);

        let list = show_.episodes.clone();
        let constructor = clone!(sender => move |ep| {
            EpisodeWidget::new(ep, &sender).container.clone()
        });

        let callback = clone!(pd, show_ => move || {
            let res = show_.set_vadjustment(&pd);
            debug_assert!(res.is_ok());
        });

        lazy_load(episodes, list.clone(), constructor, callback);

        glib::Continue(false)
    });

    Ok(())
}

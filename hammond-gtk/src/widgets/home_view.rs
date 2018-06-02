use chrono::prelude::*;
use failure::Error;

use gtk;
use gtk::prelude::*;

use crossbeam_channel::Sender;
use hammond_data::dbqueries;
use hammond_data::EpisodeWidgetQuery;
use send_cell::SendCell;

use app::Action;
use utils::{self, lazy_load_full};
use widgets::EpisodeWidget;

use std::rc::Rc;
use std::sync::Mutex;

lazy_static! {
    pub static ref EPISODES_VIEW_VALIGNMENT: Mutex<Option<SendCell<gtk::Adjustment>>> =
        Mutex::new(None);
}

#[derive(Debug, Clone)]
enum ListSplit {
    Today,
    Yday,
    Week,
    Month,
    Rest,
}

#[derive(Debug, Clone)]
pub struct HomeView {
    pub container: gtk::Box,
    scrolled_window: gtk::ScrolledWindow,
    frame_parent: gtk::Box,
    today_box: gtk::Box,
    yday_box: gtk::Box,
    week_box: gtk::Box,
    month_box: gtk::Box,
    rest_box: gtk::Box,
    today_list: gtk::ListBox,
    yday_list: gtk::ListBox,
    week_list: gtk::ListBox,
    month_list: gtk::ListBox,
    rest_list: gtk::ListBox,
}

impl Default for HomeView {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/episodes_view.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let scrolled_window: gtk::ScrolledWindow = builder.get_object("scrolled_window").unwrap();
        let frame_parent: gtk::Box = builder.get_object("frame_parent").unwrap();
        let today_box: gtk::Box = builder.get_object("today_box").unwrap();
        let yday_box: gtk::Box = builder.get_object("yday_box").unwrap();
        let week_box: gtk::Box = builder.get_object("week_box").unwrap();
        let month_box: gtk::Box = builder.get_object("month_box").unwrap();
        let rest_box: gtk::Box = builder.get_object("rest_box").unwrap();
        let today_list: gtk::ListBox = builder.get_object("today_list").unwrap();
        let yday_list: gtk::ListBox = builder.get_object("yday_list").unwrap();
        let week_list: gtk::ListBox = builder.get_object("week_list").unwrap();
        let month_list: gtk::ListBox = builder.get_object("month_list").unwrap();
        let rest_list: gtk::ListBox = builder.get_object("rest_list").unwrap();

        HomeView {
            container,
            scrolled_window,
            frame_parent,
            today_box,
            yday_box,
            week_box,
            month_box,
            rest_box,
            today_list,
            yday_list,
            week_list,
            month_list,
            rest_list,
        }
    }
}

// TODO: REFACTOR ME
impl HomeView {
    pub fn new(sender: Sender<Action>) -> Result<Rc<HomeView>, Error> {
        use self::ListSplit::*;

        let view = Rc::new(HomeView::default());
        let ignore = utils::get_ignored_shows()?;
        let episodes = dbqueries::get_episodes_widgets_filter_limit(&ignore, 100)?;
        let now_utc = Utc::now();

        let view_ = view.clone();
        let func = move |ep: EpisodeWidgetQuery| {
            let epoch = ep.epoch();
            let widget = HomeEpisode::new(&ep, &sender);

            match split(&now_utc, i64::from(epoch)) {
                Today => add_to_box(&widget, &view_.today_list, &view_.today_box),
                Yday => add_to_box(&widget, &view_.yday_list, &view_.yday_box),
                Week => add_to_box(&widget, &view_.week_list, &view_.week_box),
                Month => add_to_box(&widget, &view_.month_list, &view_.month_box),
                Rest => add_to_box(&widget, &view_.rest_list, &view_.rest_box),
            }
        };

        let view_ = view.clone();
        let callback = move || {
            view_
                .set_vadjustment()
                .map_err(|err| format!("{}", err))
                .ok();
        };

        lazy_load_full(episodes, func, callback);
        view.container.show_all();
        Ok(view)
    }

    /// Set scrolled window vertical adjustment.
    fn set_vadjustment(&self) -> Result<(), Error> {
        let guard = EPISODES_VIEW_VALIGNMENT
            .lock()
            .map_err(|err| format_err!("Failed to lock widget align mutex: {}", err))?;

        if let Some(ref sendcell) = *guard {
            // Copy the vertical scrollbar adjustment from the old view into the new one.
            sendcell
                .try_get()
                .map(|x| utils::smooth_scroll_to(&self.scrolled_window, &x));
        }

        Ok(())
    }

    /// Save the vertical scrollbar position.
    pub fn save_alignment(&self) -> Result<(), Error> {
        if let Ok(mut guard) = EPISODES_VIEW_VALIGNMENT.lock() {
            let adj = self
                .scrolled_window
                .get_vadjustment()
                .ok_or_else(|| format_err!("Could not get the adjustment"))?;
            *guard = Some(SendCell::new(adj));
            info!("Saved episodes_view alignment.");
        }

        Ok(())
    }
}

fn add_to_box(widget: &HomeEpisode, listbox: &gtk::ListBox, box_: &gtk::Box) {
    listbox.add(&widget.container);
    box_.show();
}

fn split(now: &DateTime<Utc>, epoch: i64) -> ListSplit {
    let ep = Utc.timestamp(epoch, 0);

    if now.ordinal() == ep.ordinal() && now.year() == ep.year() {
        ListSplit::Today
    } else if now.ordinal() == ep.ordinal() + 1 && now.year() == ep.year() {
        ListSplit::Yday
    } else if now.iso_week().week() == ep.iso_week().week() && now.year() == ep.year() {
        ListSplit::Week
    } else if now.month() == ep.month() && now.year() == ep.year() {
        ListSplit::Month
    } else {
        ListSplit::Rest
    }
}

#[derive(Debug, Clone)]
struct HomeEpisode {
    container: gtk::Box,
    image: gtk::Image,
    // FIXME: Change it to `EpisodeWidget` instead of a `Box`?
    episode: gtk::Box,
}

impl Default for HomeEpisode {
    fn default() -> Self {
        let builder =
            gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/episodes_view_widget.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let image: gtk::Image = builder.get_object("cover").unwrap();
        let ep = EpisodeWidget::default();
        container.pack_start(&ep.container, true, true, 6);

        HomeEpisode {
            container,
            image,
            episode: ep.container,
        }
    }
}

impl HomeEpisode {
    fn new(episode: &EpisodeWidgetQuery, sender: &Sender<Action>) -> HomeEpisode {
        let builder =
            gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/episodes_view_widget.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let image: gtk::Image = builder.get_object("cover").unwrap();
        let pid = episode.podcast_id();
        let ep = EpisodeWidget::new(episode, sender);

        let view = HomeEpisode {
            container,
            image,
            episode: ep.container.clone(),
        };

        view.init(pid);
        view
    }

    fn init(&self, podcast_id: i32) {
        self.set_cover(podcast_id)
            .map_err(|err| error!("Failed to set a cover: {}", err))
            .ok();

        self.container.pack_start(&self.episode, true, true, 6);
    }

    fn set_cover(&self, podcast_id: i32) -> Result<(), Error> {
        utils::set_image_from_path(&self.image, podcast_id, 64)
    }
}

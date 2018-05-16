use gtk;
use gtk::prelude::*;

use failure::Error;
use send_cell::SendCell;

use hammond_data::dbqueries;
use hammond_data::Podcast;

use app::Action;
use utils::{self, get_ignored_shows, lazy_load, set_image_from_path};

use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;

lazy_static! {
    static ref SHOWS_VIEW_VALIGNMENT: Mutex<Option<SendCell<gtk::Adjustment>>> = Mutex::new(None);
}

#[derive(Debug, Clone)]
pub struct ShowsView {
    pub container: gtk::Box,
    scrolled_window: gtk::ScrolledWindow,
    flowbox: gtk::FlowBox,
}

impl Default for ShowsView {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/shows_view.ui");
        let container: gtk::Box = builder.get_object("fb_parent").unwrap();
        let scrolled_window: gtk::ScrolledWindow = builder.get_object("scrolled_window").unwrap();
        let flowbox: gtk::FlowBox = builder.get_object("flowbox").unwrap();

        ShowsView {
            container,
            scrolled_window,
            flowbox,
        }
    }
}

impl ShowsView {
    pub fn new(sender: Sender<Action>) -> Result<Rc<Self>, Error> {
        let pop = Rc::new(ShowsView::default());
        pop.init(sender);
        // Populate the flowbox with the Podcasts.
        populate_flowbox(&pop)?;
        Ok(pop)
    }

    pub fn init(&self, sender: Sender<Action>) {
        self.flowbox.connect_child_activated(move |_, child| {
            on_child_activate(child, &sender)
                .map_err(|err| error!("Error along flowbox child activation: {}", err))
                .ok();
        });
    }

    /// Set scrolled window vertical adjustment.
    #[allow(unused)]
    fn set_vadjustment(&self) -> Result<(), Error> {
        let guard = SHOWS_VIEW_VALIGNMENT
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
        if let Ok(mut guard) = SHOWS_VIEW_VALIGNMENT.lock() {
            let adj = self.scrolled_window
                .get_vadjustment()
                .ok_or_else(|| format_err!("Could not get the adjustment"))?;
            *guard = Some(SendCell::new(adj));
            info!("Saved episodes_view alignment.");
        }

        Ok(())
    }
}

fn populate_flowbox(shows: &Rc<ShowsView>) -> Result<(), Error> {
    let ignore = get_ignored_shows()?;
    let podcasts = dbqueries::get_podcasts_filter(&ignore)?;

    let constructor = move |parent| ShowsChild::new(&parent).child;
    let callback = clone!(shows => move || {
         shows.set_vadjustment()
              .map_err(|err| error!("Failed to set ShowsView Alignment: {}", err))
              .ok();
     });

    let flowbox = shows.flowbox.clone();
    lazy_load(podcasts, flowbox, constructor, callback);
    Ok(())
}

fn on_child_activate(child: &gtk::FlowBoxChild, sender: &Sender<Action>) -> Result<(), Error> {
    use gtk::WidgetExt;

    // This is such an ugly hack...
    let id = WidgetExt::get_name(child)
        .ok_or_else(|| format_err!("Faild to get \"episodes\" child from the stack."))?
        .parse::<i32>()?;
    let pd = Arc::new(dbqueries::get_podcast_from_id(id)?);

    sender.send(Action::HeaderBarShowTile(pd.title().into()))?;
    sender.send(Action::ReplaceWidget(pd))?;
    sender.send(Action::ShowWidgetAnimated)?;
    Ok(())
}

#[derive(Debug)]
struct ShowsChild {
    container: gtk::Box,
    cover: gtk::Image,
    child: gtk::FlowBoxChild,
}

impl Default for ShowsChild {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/shows_child.ui");

        let container: gtk::Box = builder.get_object("fb_child").unwrap();
        let cover: gtk::Image = builder.get_object("pd_cover").unwrap();

        let child = gtk::FlowBoxChild::new();
        child.add(&container);

        ShowsChild {
            container,
            cover,
            child,
        }
    }
}

impl ShowsChild {
    pub fn new(pd: &Podcast) -> ShowsChild {
        let child = ShowsChild::default();
        child.init(pd);
        child
    }

    fn init(&self, pd: &Podcast) {
        self.container.set_tooltip_text(pd.title());
        WidgetExt::set_name(&self.child, &pd.id().to_string());

        self.set_cover(pd.id())
            .map_err(|err| error!("Failed to set a cover: {}", err))
            .ok();
    }

    fn set_cover(&self, podcast_id: i32) -> Result<(), Error> {
        set_image_from_path(&self.cover, podcast_id, 256)
    }
}

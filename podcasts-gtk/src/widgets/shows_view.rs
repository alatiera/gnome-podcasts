use gtk::{self, prelude::*, Align, SelectionMode};

use crossbeam_channel::Sender;
use failure::Error;
use fragile::Fragile;

use podcasts_data::dbqueries;
use podcasts_data::Show;

use app::Action;
use utils::{self, get_ignored_shows, lazy_load, set_image_from_path};
use widgets::BaseView;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;

lazy_static! {
    static ref SHOWS_VIEW_VALIGNMENT: Mutex<Option<Fragile<gtk::Adjustment>>> = Mutex::new(None);
}

#[derive(Debug, Clone)]
pub(crate) struct ShowsView {
    view: BaseView,
    flowbox: gtk::FlowBox,
}

impl Default for ShowsView {
    fn default() -> Self {
        let view = BaseView::default();
        let flowbox = gtk::FlowBox::new();

        flowbox.show();
        flowbox.set_vexpand(true);
        flowbox.set_hexpand(true);
        flowbox.set_row_spacing(12);
        flowbox.set_can_focus(false);
        flowbox.set_margin_top(32);
        flowbox.set_margin_bottom(32);
        flowbox.set_homogeneous(true);
        flowbox.set_column_spacing(12);
        flowbox.set_valign(Align::Start);
        flowbox.set_halign(Align::Center);
        flowbox.set_selection_mode(SelectionMode::None);
        view.add(&flowbox);

        ShowsView { view, flowbox }
    }
}

impl ShowsView {
    pub(crate) fn new(sender: Sender<Action>) -> Rc<Self> {
        let pop = Rc::new(ShowsView::default());
        pop.init(sender);
        // Populate the flowbox with the Shows.
        let res = populate_flowbox(&pop);
        debug_assert!(res.is_ok());
        pop
    }

    pub(crate) fn init(&self, sender: Sender<Action>) {
        self.flowbox.connect_child_activated(move |_, child| {
            let res = on_child_activate(child, &sender);
            debug_assert!(res.is_ok());
        });
    }

    pub(crate) fn container(&self) -> &gtk::Box {
        self.view.container()
    }

    pub(crate) fn scrolled_window(&self) -> &gtk::ScrolledWindow {
        self.view.scrolled_window()
    }

    /// Set scrolled window vertical adjustment.
    fn set_vadjustment(&self) -> Result<(), Error> {
        let guard = SHOWS_VIEW_VALIGNMENT
            .lock()
            .map_err(|err| format_err!("Failed to lock widget align mutex: {}", err))?;

        if let Some(ref fragile) = *guard {
            // Copy the vertical scrollbar adjustment from the old view into the new one.
            let res = fragile
                .try_get()
                .map(|x| utils::smooth_scroll_to(self.scrolled_window(), &x))
                .map_err(From::from);

            debug_assert!(res.is_ok());
            return res;
        }

        Ok(())
    }

    /// Save the vertical scrollbar position.
    pub(crate) fn save_alignment(&self) -> Result<(), Error> {
        if let Ok(mut guard) = SHOWS_VIEW_VALIGNMENT.lock() {
            let adj = self
                .scrolled_window()
                .get_vadjustment()
                .ok_or_else(|| format_err!("Could not get the adjustment"))?;
            *guard = Some(Fragile::new(adj));
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
        .ok_or_else(|| format_err!("Failed to get \"episodes\" child from the stack."))?
        .parse::<i32>()?;
    let pd = Arc::new(dbqueries::get_podcast_from_id(id)?);

    sender.send(Action::HeaderBarShowTile(pd.title().into()));
    sender.send(Action::ReplaceWidget(pd));
    sender.send(Action::ShowWidgetAnimated);
    Ok(())
}

#[derive(Debug, Clone)]
struct ShowsChild {
    cover: gtk::Image,
    child: gtk::FlowBoxChild,
}

impl Default for ShowsChild {
    fn default() -> Self {
        let cover = gtk::Image::new_from_icon_name("image-x-generic-symbolic", -1);
        let child = gtk::FlowBoxChild::new();

        cover.set_pixel_size(256);
        child.add(&cover);
        child.show_all();

        ShowsChild { cover, child }
    }
}

impl ShowsChild {
    pub(crate) fn new(pd: &Show) -> ShowsChild {
        let child = ShowsChild::default();
        child.init(pd);
        child
    }

    fn init(&self, pd: &Show) {
        self.child.set_tooltip_text(pd.title());
        WidgetExt::set_name(&self.child, &pd.id().to_string());

        self.set_cover(pd.id())
    }

    fn set_cover(&self, show_id: i32) {
        // The closure above is a regular `Fn` closure.
        // which means we can't mutate stuff inside it easily,
        // so Cell is used.
        //
        // `Option<T>` along with the `.take()` method ensure
        // that the function will only be run once, during the first execution.
        let show_id = Cell::new(Some(show_id));

        self.cover.connect_draw(move |cover, _| {
            show_id.take().map(|id| {
                set_image_from_path(cover, id, 256)
                    .map_err(|err| error!("Failed to set a cover: {}", err))
                    .ok();
            });

            gtk::Inhibit(false)
        });
    }
}

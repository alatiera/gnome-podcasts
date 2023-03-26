// shows_view.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use adw::{prelude::BinExt, subclass::prelude::*};
use glib::clone;
use gtk::glib;
use gtk::{prelude::*, Adjustment, Align, SelectionMode};

use anyhow::Result;
use glib::Sender;

use podcasts_data::dbqueries;
use podcasts_data::Show;

use crate::app::Action;
use crate::utils::{get_ignored_shows, lazy_load_flowbox, set_image_from_path};
use crate::widgets::BaseView;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct ShowsView {
    pub(crate) view: BaseView,
    flowbox: gtk::FlowBox,
}

impl Default for ShowsView {
    fn default() -> Self {
        let view = BaseView::default();
        let flowbox = gtk::FlowBox::new();

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
        view.set_content(&flowbox);

        ShowsView { view, flowbox }
    }
}

impl ShowsView {
    pub(crate) fn new(sender: Sender<Action>, vadj: Option<Adjustment>) -> Rc<Self> {
        let pop = Rc::new(ShowsView::default());
        pop.init(sender);
        // Populate the flowbox with the Shows.
        let res = populate_flowbox(&pop, vadj);
        debug_assert!(res.is_ok());
        pop
    }

    pub(crate) fn init(&self, sender: Sender<Action>) {
        self.flowbox.connect_child_activated(move |_, child| {
            let res = on_child_activate(child, &sender);
            debug_assert!(res.is_ok());
        });
    }
}

fn populate_flowbox(shows: &Rc<ShowsView>, vadj: Option<Adjustment>) -> Result<()> {
    let ignore = get_ignored_shows()?;
    let podcasts = dbqueries::get_podcasts_filter(&ignore)?;
    let flowbox_weak = shows.flowbox.downgrade();

    let constructor = move |parent| ShowsChild::new(&parent).row;
    let callback = clone!(@weak shows => move || {
        if vadj.is_some() {
            shows.view.set_adjustments(None, vadj.as_ref())
        }
    });

    lazy_load_flowbox(podcasts, flowbox_weak, constructor, callback);
    Ok(())
}

fn on_child_activate(child: &gtk::FlowBoxChild, sender: &Sender<Action>) -> Result<()> {
    // This is such an ugly hack...
    let id = child
        .child()
        .unwrap()
        .downcast::<ShowCover>()
        .expect("Could not downcast Widget to PdShowCover")
        .id();
    let pd = Arc::new(dbqueries::get_podcast_from_id(id)?);

    send!(sender, Action::HeaderBarShowTile(pd.title().into()));
    send!(sender, Action::ReplaceWidget(pd));
    send!(sender, Action::ShowWidgetAnimated);
    Ok(())
}

#[derive(Debug)]
pub struct ShowCoverPrivate {
    pub cover: gtk::Image,
    pub show_id: Cell<i32>,
}

#[glib::object_subclass]
impl ObjectSubclass for ShowCoverPrivate {
    const NAME: &'static str = "PdShowCover";
    type Type = ShowCover;
    type ParentType = adw::Bin;

    fn new() -> Self {
        Self {
            // FIXME: bundle the symbolic in resources
            cover: gtk::Image::from_icon_name("image-x-generic-symbolic"),
            // cover: gtk::Picture::new(),
            show_id: Cell::default(),
        }
    }
}

impl ObjectImpl for ShowCoverPrivate {
    fn constructed(&self) {
        self.parent_constructed();
        self.cover.set_pixel_size(256);
        self.cover.add_css_class("rounded-big");
        self.cover.set_overflow(gtk::Overflow::Hidden);

        self.obj().set_child(Some(&self.cover));
    }
}

impl WidgetImpl for ShowCoverPrivate {}
impl BinImpl for ShowCoverPrivate {}

glib::wrapper! {
    pub struct ShowCover(ObjectSubclass<ShowCoverPrivate>)
        @extends gtk::Widget, adw::Bin;
}

impl ShowCover {
    fn new() -> Self {
        glib::Object::new()
    }

    fn set_id(&self, id: i32) {
        self.imp().show_id.set(id);
    }

    fn id(&self) -> i32 {
        self.imp().show_id.get()
    }

    fn load_image(&self) -> Result<()> {
        let self_ = self.imp();
        set_image_from_path(&self_.cover, self_.show_id.get(), 256)?;
        Ok(())
    }
}

#[derive(Debug)]
struct ShowsChild {
    cover: ShowCover,
    row: gtk::FlowBoxChild,
}

impl ShowsChild {
    fn new(pd: &Show) -> ShowsChild {
        let cover = ShowCover::new();
        let row = gtk::FlowBoxChild::new();
        row.set_child(Some(&cover));

        let child = ShowsChild { cover, row };
        child.init(pd);
        child
    }

    fn init(&self, pd: &Show) {
        self.row.set_tooltip_text(Some(pd.title()));
        self.cover.set_id(pd.id());

        self.cover.set_id(pd.id());
        self.set_cover();
    }

    fn set_cover(&self) {
        self.cover
            .load_image()
            .map_err(|err| error!("Failed to set a cover: {}", err))
            .ok();
    }
}

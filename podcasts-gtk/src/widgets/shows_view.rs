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

use glib::clone;
use gtk::{self, prelude::*, Adjustment, Align, SelectionMode};

use anyhow::Result;
use glib::Sender;

use podcasts_data::dbqueries;
use podcasts_data::Show;

use crate::app::Action;
use crate::utils::{get_ignored_shows, lazy_load, set_image_from_path};
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

    let constructor = move |parent| ShowsChild::new(&parent).child;
    let callback = clone!(@weak shows => move || {
        if vadj.is_some() {
            shows.view.set_adjustments(None, vadj.as_ref())
        }
    });

    lazy_load(podcasts, flowbox_weak, constructor, callback);
    Ok(())
}

fn on_child_activate(child: &gtk::FlowBoxChild, sender: &Sender<Action>) -> Result<()> {
    // This is such an ugly hack...
    let id = child.get_widget_name().parse::<i32>()?;
    let pd = Arc::new(dbqueries::get_podcast_from_id(id)?);

    send!(sender, Action::HeaderBarShowTile(pd.title().into()));
    send!(sender, Action::ReplaceWidget(pd));
    send!(sender, Action::ShowWidgetAnimated);
    Ok(())
}

#[derive(Debug, Clone)]
struct ShowsChild {
    cover: gtk::Image,
    child: gtk::FlowBoxChild,
}

impl Default for ShowsChild {
    fn default() -> Self {
        let cover = gtk::Image::from_icon_name(
            Some("image-x-generic-symbolic"),
            gtk::IconSize::__Unknown(-1),
        );
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
        self.child.set_tooltip_text(Some(pd.title()));
        self.child.set_widget_name(&pd.id().to_string());

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
            if let Some(id) = show_id.take() {
                set_image_from_path(cover, id, 256)
                    .map_err(|err| error!("Failed to set a cover: {}", err))
                    .ok();
            }

            gtk::Inhibit(false)
        });
    }
}

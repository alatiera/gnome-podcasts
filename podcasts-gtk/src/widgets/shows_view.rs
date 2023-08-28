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
use gtk::glib;
use gtk::{prelude::*, Adjustment, Align, SelectionMode};

use anyhow::Result;

use podcasts_data::dbqueries;
use podcasts_data::Show;

use crate::utils::{get_ignored_shows, lazy_load_flowbox};
use crate::widgets::BaseView;

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
        flowbox.set_can_focus(true);
        flowbox.set_margin_top(32);
        flowbox.set_margin_bottom(32);
        flowbox.set_homogeneous(true);
        flowbox.set_column_spacing(12);
        flowbox.set_valign(Align::Start);
        flowbox.set_halign(Align::Center);
        flowbox.set_selection_mode(SelectionMode::None);
        flowbox.update_property(&[gtk::accessible::Property::Label("Shows")]);
        view.set_content(&flowbox);

        ShowsView { view, flowbox }
    }
}

impl ShowsView {
    pub(crate) fn new(vadj: Option<Adjustment>) -> Self {
        let pop = ShowsView::default();
        let res = populate_flowbox(&pop, vadj);
        debug_assert!(res.is_ok());
        pop
    }
}

fn populate_flowbox(shows: &ShowsView, vadj: Option<Adjustment>) -> Result<()> {
    let ignore = get_ignored_shows()?;
    let podcasts = dbqueries::get_podcasts_filter(&ignore)?;
    let flowbox_weak = shows.flowbox.downgrade();

    let constructor = move |podcast: Show| {
        let widget = gtk::FlowBoxChild::new();
        let button = gtk::Button::new();
        let image = gtk::Image::from_icon_name("image-x-generic-symbolic");

        image.set_pixel_size(256);
        image.add_css_class("rounded-big");
        image.set_overflow(gtk::Overflow::Hidden);

        let result = crate::utils::set_image_from_path(&image, podcast.id(), 256);
        if let Err(e) = result {
            error!("Failed to load cover for {}: {e}", podcast.title());
        }
        button.set_child(Some(&image));
        button.set_action_name(Some("app.go-to-show"));
        button.set_action_target_value(Some(&podcast.id().to_variant()));
        button.set_tooltip_text(Some(podcast.title()));
        button.add_css_class("flat");
        button.add_css_class("show_button");
        button.set_can_focus(false);
        widget.set_child(Some(&button));

        widget.set_tooltip_text(Some(podcast.title()));
        widget.connect_activate(clone!(@weak button => move |_| {
            button.activate();
        }));
        widget
    };
    let callback = clone!(@weak shows.view as view => move || {
        if vadj.is_some() {
            view.set_adjustments(None, vadj.as_ref())
        }
    });

    lazy_load_flowbox(podcasts, flowbox_weak, constructor, callback);
    Ok(())
}

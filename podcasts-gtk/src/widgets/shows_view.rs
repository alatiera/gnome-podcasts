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

use adw::subclass::prelude::*;
use glib::clone;
use gtk::glib;
use gtk::{prelude::*, Adjustment, Align, SelectionMode};

use anyhow::Result;

use podcasts_data::dbqueries;
use podcasts_data::Show;

use crate::i18n::i18n;
use crate::utils::{get_ignored_shows, lazy_load_full};
use crate::widgets::BaseView;

#[derive(Debug, Default)]
pub struct ShowsViewPriv {
    view: BaseView,
    flowbox: gtk::FlowBox,
}

#[glib::object_subclass]
impl ObjectSubclass for ShowsViewPriv {
    const NAME: &'static str = "PdShowsView";
    type Type = super::ShowsView;
    type ParentType = adw::Bin;
}

impl ObjectImpl for ShowsViewPriv {
    fn constructed(&self) {
        self.parent_constructed();

        self.flowbox.set_vexpand(true);
        self.flowbox.set_hexpand(true);
        self.flowbox.set_row_spacing(12);
        self.flowbox.set_can_focus(true);
        self.flowbox.set_margin_top(32);
        self.flowbox.set_margin_bottom(32);
        self.flowbox.set_homogeneous(true);
        self.flowbox.set_column_spacing(12);
        self.flowbox.set_valign(Align::Start);
        self.flowbox.set_halign(Align::Center);
        self.flowbox.set_selection_mode(SelectionMode::None);
        self.flowbox
            .update_property(&[gtk::accessible::Property::Label(&i18n("Shows"))]);
        self.view.set_content(&self.flowbox);
    }
}

impl WidgetImpl for ShowsViewPriv {}
impl BinImpl for ShowsViewPriv {}

impl ShowsViewPriv {
    fn populate_flowbox(&self, vadj: Option<Adjustment>) -> Result<()> {
        let ignore = get_ignored_shows()?;
        let podcasts = dbqueries::get_podcasts_filter(&ignore)?;

        let callback = clone!(@weak self.view as view => move || {
            if vadj.is_some() {
                view.set_adjustments(None, vadj.as_ref())
            }
        });

        let container = self.flowbox.downgrade();
        let insert = move |widget: gtk::Widget| {
            let container = match container.upgrade() {
                Some(c) => c,
                None => return,
            };

            container.append(&widget);
            widget.set_visible(true);
        };

        lazy_load_full(podcasts, create_show_child, insert, callback);

        Ok(())
    }
}

// TODO: Make this a widget
fn create_show_child(podcast: Show) -> gtk::Widget {
    let widget = gtk::FlowBoxChild::new();
    let button = gtk::Button::new();
    let image = gtk::Image::from_icon_name("image-x-generic-symbolic");

    image.set_pixel_size(256);
    image.add_css_class("rounded-big");
    image.set_overflow(gtk::Overflow::Hidden);
    button.set_child(Some(&image));

    let pd = podcast.clone();
    glib::idle_add_local(
        clone!(@weak image => @default-return glib::ControlFlow::Break, move || {
            let result = crate::utils::set_image_from_path(&image, pd.id(), 256);
            if let Err(e) = result {
                error!("Failed to load cover for {}: {e}", pd.title());
            }
            glib::ControlFlow::Break
        }),
    );

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
    widget.into()
}

glib::wrapper! {
    pub struct ShowsView(ObjectSubclass<ShowsViewPriv>)
        @extends gtk::Widget, adw::Bin;
}

impl Default for ShowsView {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ShowsView {
    pub fn new(vadj: Option<Adjustment>) -> Self {
        let pop = ShowsView::default();
        let res = pop.imp().populate_flowbox(vadj);
        debug_assert!(res.is_ok());
        pop
    }

    pub fn view(&self) -> &BaseView {
        &self.imp().view
    }
}

// shows_view.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
// Copyright 2024 nee <nee-git@patchouli.garden>
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

use adw::prelude::*;
use adw::subclass::prelude::*;
use anyhow::{anyhow, Result};
use async_channel::Sender;
use glib::clone;
use glib::object::Object;
use gtk::gio;
use gtk::glib;
use std::cell::Cell;
use std::cell::RefCell;
use std::cell::RefMut;
use std::sync::Arc;

use crate::app::Action;
use crate::download_covers::load_widget_texture;
use crate::i18n::i18n;
use crate::utils::get_ignored_shows;
use crate::widgets::BaseView;
use podcasts_data::dbqueries;
use podcasts_data::{Show, ShowId};

#[derive(Debug, Default)]
pub struct ShowsViewPriv {
    view: BaseView,
    grid: gtk::GridView,
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
        let missing_icon = load_missing_icon();
        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(clone!(
            #[strong]
            missing_icon,
            move |_factory, item| {
                let item = item.downcast_ref::<gtk::ListItem>().unwrap();
                // TODO: Make this a widget with completed/fetch-error info overlays
                let picture = gtk::Picture::builder()
                    .width_request(150)
                    .height_request(150)
                    .can_focus(false)
                    .build();
                picture.set_paintable(missing_icon.as_ref());
                picture.add_css_class("flat");
                picture.add_css_class("rounded-big");
                picture.add_css_class("show-button");
                picture.add_css_class("shows-view-cover");
                picture.set_content_fit(gtk::ContentFit::ScaleDown);

                item.set_child(Some(&picture));
            }
        ));
        factory.connect_bind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let data = item.item().and_downcast::<ShowCoverModel>().unwrap();
            let child = item.child().and_downcast::<gtk::Picture>().unwrap();

            let id = data.show_id();
            let load_handle = load_widget_texture(&child, id, crate::Thumb256);
            let mut load_handle_store = data.get_mut_load_handle();
            *load_handle_store = Some(load_handle);
        });
        factory.connect_unbind(move |_factory, item| {
            let item = item.downcast_ref::<gtk::ListItem>().unwrap();
            let data = item.item().and_downcast::<ShowCoverModel>().unwrap();
            let child = item.child().and_downcast::<gtk::Picture>().unwrap();
            // cancel loading the picture
            if let Some(handle) = data.get_mut_load_handle().take() {
                handle.abort();
            }
            child.set_paintable(missing_icon.as_ref());
        });

        self.grid.set_factory(Some(&factory));
        self.grid.set_single_click_activate(true);
        self.grid.set_can_focus(true);
        self.grid.set_vexpand(true);
        self.grid.set_hexpand(true);
        self.grid.set_min_columns(2);
        self.grid.set_max_columns(7);
        self.grid.set_valign(gtk::Align::Fill);
        self.grid.set_halign(gtk::Align::Fill);
        self.grid.set_height_request(500);
        // makes tabbing down to the player widget is easier.
        self.grid.set_tab_behavior(gtk::ListTabBehavior::Item);
        self.grid.add_css_class("shows-grid");
        self.grid.set_vscroll_policy(gtk::ScrollablePolicy::Natural);
        self.grid
            // Translators: Shows as a noun, meaning Podcast-Shows.
            .update_property(&[gtk::accessible::Property::Label(&i18n("Shows"))]);

        let clamp = adw::ClampScrollable::builder()
            .child(&self.grid)
            .valign(gtk::Align::Fill)
            .halign(gtk::Align::Fill)
            .vscroll_policy(gtk::ScrollablePolicy::Natural)
            .orientation(gtk::Orientation::Horizontal)
            .maximum_size((256 + 6 + 6) * 7) // picture + paddings * max_columns
            .build();
        self.view.set_content(&clamp);
        self.obj().set_child(Some(&self.view));
    }
}

fn load_missing_icon() -> Option<gtk::IconPaintable> {
    let display = gtk::gdk::Display::default()?;
    // get the max scale form any of the monitors
    let scale = display.monitors().into_iter().fold(1, |acc, m| {
        let m_scale = (|| Some(m.ok()?.downcast::<gtk::gdk::Monitor>().ok()?.scale_factor()))()
            .unwrap_or(acc);
        std::cmp::max(acc, m_scale)
    });
    let theme = gtk::IconTheme::for_display(&display);
    if theme.has_icon("image-x-generic-symbolic") {
        Some(theme.lookup_icon(
            "image-x-generic-symbolic",
            &[],
            128, // 1/2 size of picture to get padding
            scale,
            gtk::TextDirection::Ltr,
            gtk::IconLookupFlags::FORCE_SYMBOLIC,
        ))
    } else {
        None
    }
}

impl WidgetImpl for ShowsViewPriv {}
impl BinImpl for ShowsViewPriv {}

impl ShowsViewPriv {
    fn set_data(&self) {
        let this = self.downgrade();
        crate::MAINCONTEXT.spawn_local_with_priority(
            glib::source::Priority::DEFAULT_IDLE,
            async move {
                let data = gio::spawn_blocking(get_episodes).await;
                if let Ok(Ok(podcasts)) = data {
                    let model = gio::ListStore::new::<ShowCoverModel>();
                    for pod in podcasts {
                        let item = ShowCoverModel::new(pod.id());
                        model.append(&item);
                    }
                    if let Some(this) = this.upgrade() {
                        let selection_model = gtk::NoSelection::new(Some(model));
                        this.grid.set_model(Some(&selection_model));
                    }
                }
            },
        );
    }
}

fn get_episodes() -> Result<Vec<Show>> {
    let ignore = get_ignored_shows()?;
    let podcasts = dbqueries::get_podcasts_filter(&ignore)?;
    Ok(podcasts)
}

glib::wrapper! {
    pub struct ShowsView(ObjectSubclass<ShowsViewPriv>)
        @extends gtk::Widget, adw::Bin;
}

impl ShowsView {
    pub(crate) fn new(sender: Sender<Action>) -> Self {
        let this: Self = glib::Object::new();
        this.imp().set_data();
        this.imp().grid.connect_activate(move |gridview, index| {
            if let Err(err) = on_child_activate(gridview, index, &sender) {
                error!("Failed to activated ShowCover {err}");
            }
        });

        this
    }

    pub fn update_model(&self) {
        self.imp().set_data();
    }
}

fn on_child_activate(gridview: &gtk::GridView, index: u32, sender: &Sender<Action>) -> Result<()> {
    let id = gridview
        .model()
        .ok_or(anyhow!("no model in gridview"))?
        .item(index)
        .ok_or(anyhow!("clicked show not found in gridview model"))?
        .downcast::<ShowCoverModel>()
        .unwrap()
        .show_id();
    let pd = Arc::new(dbqueries::get_podcast_from_id(id)?);
    send_blocking!(sender, Action::GoToShow(pd));
    Ok(())
}

// Model data type
// -------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct ShowCoverModelPrivate {
    pub show_id: Cell<i32>,
    pub load_handle: RefCell<Option<glib::JoinHandle<()>>>,
}

#[glib::object_subclass]
impl ObjectSubclass for ShowCoverModelPrivate {
    const NAME: &'static str = "PdShowCoverModel";
    type Type = ShowCoverModel;
    type ParentType = Object;
}

impl ObjectImpl for ShowCoverModelPrivate {}

gtk::glib::wrapper! {
    pub struct ShowCoverModel(ObjectSubclass<ShowCoverModelPrivate>);
}

impl ShowCoverModel {
    pub(crate) fn new(id: ShowId) -> Self {
        let self_: Self = glib::Object::new();
        self_.imp().show_id.set(id.0);
        self_
    }

    fn show_id(&self) -> ShowId {
        ShowId(self.imp().show_id.get())
    }

    fn get_mut_load_handle(&self) -> RefMut<Option<glib::JoinHandle<()>>> {
        self.imp().load_handle.borrow_mut()
    }
}
// -------------------------------------------------------------------

// show.rs
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

use adw::prelude::*;
use adw::subclass::prelude::*;
use anyhow::Result;
use async_channel::Sender;
use glib::clone;
use gtk::gio;
use gtk::glib;
use gtk::CompositeTemplate;
use std::cell::Cell;
use std::sync::Arc;

use crate::app::Action;
use crate::utils::lazy_load;
use crate::widgets::{EmptyShow, EpisodeWidget, ReadMoreLabel, ShowMenu};
use podcasts_data::dbqueries;
use podcasts_data::{EpisodeModel, EpisodeWidgetModel, Show, ShowId};

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/gnome/Podcasts/gtk/show_widget.ui")]
pub struct ShowWidgetPriv {
    #[template_child]
    pub cover: TemplateChild<gtk::Image>,
    #[template_child]
    pub read_more_label: TemplateChild<ReadMoreLabel>,
    #[template_child]
    pub episodes: TemplateChild<gtk::ListBox>,
    #[template_child]
    pub(crate) view: TemplateChild<gtk::Widget>,
    #[template_child]
    pub(crate) secondary_menu: TemplateChild<gtk::MenuButton>,

    pub show_id: Cell<Option<ShowId>>,
}

impl ShowWidgetPriv {
    fn init(&self) {
        self.read_more_label.init();
        self.read_more_label
            .update_property(&[gtk::accessible::Property::Description(
                "Podcast Description",
            )]);
    }

    /// Set the description text.
    fn set_description(&self, text: &str) {
        let markup = html2text::config::plain()
            .string_from_read(text.as_bytes(), text.as_bytes().len())
            .unwrap_or_else(|_| text.to_string());
        let markup = markup.trim();
        if !markup.is_empty() {
            self.read_more_label.set_label(markup);
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ShowWidgetPriv {
    const NAME: &'static str = "PdShowWidget";
    type Type = super::ShowWidget;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        Self::bind_template(klass);
        klass.set_layout_manager_type::<gtk::BinLayout>();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ShowWidgetPriv {
    fn dispose(&self) {
        self.view.unparent();
    }
}

impl WidgetImpl for ShowWidgetPriv {}

glib::wrapper! {
    pub struct ShowWidget(ObjectSubclass<ShowWidgetPriv>)
        @extends gtk::Widget;
}

impl Default for ShowWidget {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ShowWidget {
    pub(crate) fn new(pd: Arc<Show>, sender: &Sender<Action>) -> ShowWidget {
        let widget = ShowWidget::default();
        widget.init(&pd);

        let menu = ShowMenu::new(&pd, &widget.imp().episodes, sender);
        widget.imp().secondary_menu.set_menu_model(Some(&menu.menu));

        let res = populate_listbox(&widget, pd, sender);
        debug_assert!(res.is_ok());

        widget
    }

    pub(crate) fn init(&self, pd: &Arc<Show>) {
        let self_ = self.imp();
        self_.init();
        self_.set_description(pd.description());
        self_.show_id.set(Some(pd.id()));
        self.set_cover(pd);
    }

    /// Set the show cover.
    fn set_cover(&self, pd: &Arc<Show>) {
        crate::download_covers::load_widget_texture(
            &self.imp().cover.get(),
            pd.id(),
            crate::Thumb256,
        );
    }

    pub(crate) fn show_id(&self) -> Option<ShowId> {
        self.imp().show_id.get()
    }
}

/// Populate the listbox with the shows episodes.
fn populate_listbox(show: &ShowWidget, pd: Arc<Show>, sender: &Sender<Action>) -> Result<()> {
    let count = dbqueries::get_pd_episodes_count(&pd)?;

    if count == 0 {
        let empty = EmptyShow::default();
        show.imp().episodes.append(&empty);
        return Ok(());
    }

    let constructor = clone!(
        #[strong]
        sender,
        move |ep: EpisodeWidgetModel| {
            let id = ep.id();
            let episode_widget = EpisodeWidget::new(&sender, ep, false);
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&episode_widget));
            row.set_action_name(Some("app.go-to-episode"));
            row.set_action_target_value(Some(&id.0.to_variant()));
            row.upcast()
        }
    );

    let listbox = show.imp().episodes.upcast_ref::<gtk::Widget>().downgrade();
    crate::MAINCONTEXT.spawn_local_with_priority(
        glib::source::Priority::DEFAULT_IDLE,
        async move {
            let episodes = gio::spawn_blocking(clone!(
                #[strong]
                pd,
                move || dbqueries::get_pd_episodeswidgets(&pd),
            ));

            if let Ok(Ok(episodes)) = episodes.await {
                let results = lazy_load(episodes, listbox, constructor).await;
                for result in results {
                    if let Err(e) = result {
                        log::error!("Error: {:?}", e);
                    }
                }
            }
        },
    );

    Ok(())
}

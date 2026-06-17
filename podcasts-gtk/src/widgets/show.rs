// show.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
// Copyright 2020-2026 nee <nee-git@patchouli.garden>
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
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;
use std::cell::{Cell, RefCell};
use std::sync::Arc;

use crate::app::Action;
use crate::utils::lazy_load;
use crate::widgets::{
    EmptyShow, EpisodeWidget, FilterMenu, FilterMenuMode, ReadMoreLabel, ShowMenu,
};
use podcasts_data::dbqueries;
use podcasts_data::dbqueries::EpisodeFilter;
use podcasts_data::{EpisodeModel, EpisodeWidgetModel, Show, ShowId};

#[derive(Debug, Default, CompositeTemplate, glib::Properties)]
#[template(resource = "/org/gnome/Podcasts/gtk/show_widget.ui")]
#[properties(wrapper_type = ShowWidget)]
pub struct ShowWidgetPriv {
    #[template_child]
    pub cover: TemplateChild<gtk::Image>,
    #[template_child]
    pub read_more_label: TemplateChild<ReadMoreLabel>,
    #[template_child]
    pub episodes_container: TemplateChild<adw::Bin>,
    #[template_child]
    pub(crate) view: TemplateChild<gtk::Widget>,
    #[template_child]
    pub(crate) secondary_menu: TemplateChild<gtk::MenuButton>,
    #[template_child]
    pub(crate) filter_menu: TemplateChild<FilterMenu>,
    #[template_child]
    search_bar: TemplateChild<gtk::SearchBar>,
    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,

    pub episodes: RefCell<gtk::ListBox>,

    pub show_id: Cell<Option<ShowId>>,

    #[property(set, get)]
    pub(crate) is_mobile_layout: Cell<bool>,
    #[property(set, get)]
    pub(crate) description_width: Cell<i32>,
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
            .string_from_read(text.as_bytes(), text.len())
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

#[glib::derived_properties]
impl ObjectImpl for ShowWidgetPriv {
    fn dispose(&self) {
        self.view.unparent();
    }
}

impl WidgetImpl for ShowWidgetPriv {}

glib::wrapper! {
    pub struct ShowWidget(ObjectSubclass<ShowWidgetPriv>)
        @extends gtk::Widget,
    @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl Default for ShowWidget {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ShowWidget {
    pub(crate) fn new(pd: Arc<Show>, sender: &Sender<Action>) -> ShowWidget {
        let widget = ShowWidget::default();

        widget.init(&pd, sender);

        let menu = ShowMenu::new(&pd, &widget, sender);
        widget.imp().secondary_menu.set_menu_model(Some(&menu.menu));

        let res = widget.populate_listbox(pd, sender);
        debug_assert!(res.is_ok());

        widget
    }

    pub(crate) fn init(&self, pd: &Arc<Show>, sender: &Sender<Action>) {
        let imp = self.imp();
        imp.init();
        imp.set_description(pd.description());
        imp.show_id.set(Some(pd.id()));

        imp.filter_menu.init(FilterMenuMode::Episode);
        imp.filter_menu.connect_filter_changed(glib::clone!(
            #[strong]
            sender,
            #[weak(rename_to = this)]
            self,
            move |_| this.reload(&sender)
        ));
        imp.filter_menu
            .search_button()
            .bind_property("active", &imp.search_bar.get(), "search-mode-enabled")
            .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
            .build();

        imp.search_bar.set_key_capture_widget(Some(&imp.view.get()));

        imp.search_entry.connect_search_changed(clone!(
            #[strong]
            sender,
            #[weak(rename_to = this)]
            self,
            move |_| this.reload(&sender)
        ));

        self.set_cover(pd);

        self.bind_property("is_mobile_layout", self, "description_width")
            .transform_to(move |_, is_mobile: bool| Some(if is_mobile { 320 } else { 600 }))
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
    }

    pub(crate) fn reload(&self, sender: &Sender<Action>) {
        if let Ok(pd) = dbqueries::get_podcast_from_id(self.show_id().unwrap()) {
            let res = self.populate_listbox(Arc::new(pd), sender);
            debug_assert!(res.is_ok());
        }
    }

    /// Set the show cover.
    fn set_cover(&self, pd: &Arc<Show>) {
        crate::download_covers::load_widget_texture(
            &self.imp().cover.get(),
            pd.id(),
            crate::Thumb256,
            true,
        );
    }

    pub(crate) fn open_search(&self) {
        self.imp().search_bar.set_search_mode(true);
    }

    pub fn episode_filter(&self) -> EpisodeFilter {
        let mut filter = self.imp().filter_menu.episode_filter();
        let search = self.imp().search_entry.text();
        if !search.is_empty() {
            filter.search = Some(search.to_string());
        }
        filter
    }

    pub(crate) fn show_id(&self) -> Option<ShowId> {
        self.imp().show_id.get()
    }

    pub(crate) fn update_episode(&self, ep: &EpisodeWidgetModel) {
        let imp = self.imp();
        let id = ep.id();
        let mut i = 0;
        while let Some(row) = imp.episodes.borrow().row_at_index(i) {
            if let Some(Ok(episode)) = row.child().map(|w| w.downcast::<EpisodeWidget>())
                && episode.id() == id
            {
                episode.update_episode_state(ep);
                return;
            }
            i += 1;
        }
    }

    pub(crate) fn mark_all_played(&self) {
        let imp = self.imp();
        let mut i = 0;
        while let Some(row) = imp.episodes.borrow().row_at_index(i) {
            if let Some(Ok(episode)) = row.child().map(|w| w.downcast::<EpisodeWidget>()) {
                episode.set_played(true);
            }
            i += 1;
        }
    }

    fn make_empty(&self, due_to_filter: bool) {
        let empty = EmptyShow::default();
        let list = gtk::ListBox::new();
        list.add_css_class("content");
        if due_to_filter {
            empty.set_empty_because_of_filter();
        }
        list.append(&empty);
        self.imp().episodes_container.set_child(Some(&list));
        self.imp().episodes.replace(list);
    }

    /// Populate the listbox with the shows episodes.
    fn populate_listbox(&self, pd: Arc<Show>, sender: &Sender<Action>) -> Result<()> {
        let count = dbqueries::get_pd_episodes_count(&pd)?;
        if count == 0 {
            self.make_empty(false);
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

        let listbox = gtk::ListBox::new();
        listbox.add_css_class("content");
        self.imp().episodes_container.set_child(Some(&listbox));
        let listbox_weak = listbox.upcast_ref::<gtk::Widget>().downgrade();
        self.imp().episodes.replace(listbox);
        let filter = self.episode_filter();
        crate::MAINCONTEXT.spawn_local_with_priority(
            glib::source::Priority::DEFAULT_IDLE,
            glib::clone!(
                #[weak(rename_to = this)]
                self,
                async move {
                    let episodes = gio::spawn_blocking(clone!(
                        #[strong]
                        pd,
                        move || dbqueries::get_pd_episode_widgets(&pd, &filter),
                    ));

                    if let Ok(Ok(episodes)) = episodes.await {
                        if episodes.is_empty() {
                            this.make_empty(true);
                            return;
                        }
                        let results = lazy_load(episodes, listbox_weak, constructor).await;
                        for result in results {
                            if let Err(e) = result {
                                log::error!("Error failed to load show episodes: {:?}", e);
                            }
                        }
                    }
                }
            ),
        );

        Ok(())
    }
}

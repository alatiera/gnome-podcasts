// filter_menu.rs
//
// Copyright 2024-2026 nee <nee-git@patchouli.garden>
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
use glib::subclass::InitializingObject;
use gtk::gio;
use gtk::{CompositeTemplate, glib, glib::clone, glib::subclass::Signal, prelude::*};
use std::cell::RefCell;
use std::sync::OnceLock;

use podcasts_data::dbqueries::{EpisodeFilter, ShowFilter};

pub enum FilterMenuMode {
    Episode,
    Show,
}

/// Ui for the filter menu for both Shows and Episodes.
/// Uses a toggle to flip into shows mode.
#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/filter_menu.ui")]
pub struct FilterMenuPriv {
    #[template_child]
    filter_menu_episodes: TemplateChild<gio::MenuModel>,
    #[template_child]
    filter_menu_shows: TemplateChild<gio::MenuModel>,
    #[template_child]
    button: TemplateChild<gtk::MenuButton>,
    #[template_child]
    is_active_circle: TemplateChild<gtk::Box>,
    #[template_child]
    search_button: TemplateChild<gtk::ToggleButton>,

    group: RefCell<gio::SimpleActionGroup>,
}

#[glib::object_subclass]
impl ObjectSubclass for FilterMenuPriv {
    const NAME: &'static str = "PdFilterMenu";
    type Type = FilterMenu;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for FilterMenuPriv {}
impl ObjectImpl for FilterMenuPriv {
    fn signals() -> &'static [Signal] {
        static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
        SIGNALS.get_or_init(|| vec![Signal::builder("filter-changed").build()])
    }
}
impl BinImpl for FilterMenuPriv {}

glib::wrapper! {
    pub struct FilterMenu(ObjectSubclass<FilterMenuPriv>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for FilterMenu {
    fn default() -> Self {
        let this: Self = glib::Object::new();
        this
    }
}

impl FilterMenu {
    pub(crate) fn init(&self, mode: FilterMenuMode) {
        let group = self.imp().group.borrow();
        let order = gio::SimpleAction::new_stateful(
            "order",
            Some(glib::VariantTy::STRING),
            &"default".into(),
        );
        let played =
            gio::SimpleAction::new_stateful("played", Some(glib::VariantTy::STRING), &"all".into());
        let downloaded = gio::SimpleAction::new_stateful(
            "downloaded",
            Some(glib::VariantTy::STRING),
            &"all".into(),
        );
        self.setup_change_action(&order);
        self.setup_change_action(&played);
        self.setup_change_action(&downloaded);

        group.add_action(&order);
        group.add_action(&played);
        group.add_action(&downloaded);

        self.insert_action_group("filter", Some(&group.clone()));

        let menu = match mode {
            FilterMenuMode::Show => self.imp().filter_menu_shows.get(),
            FilterMenuMode::Episode => self.imp().filter_menu_episodes.get(),
        };
        self.imp().button.set_menu_model(Some(&menu));
    }

    fn setup_change_action(&self, action: &gio::SimpleAction) {
        action.connect_change_state(clone!(
            #[weak(rename_to = this)]
            self,
            move |a, value| {
                if let Some(value) = value {
                    a.set_state(value);
                }
                this.emit_changed()
            }
        ));
    }

    fn emit_changed(&self) {
        self.emit_by_name::<()>("filter-changed", &[]);
        self.update_active_css();
    }

    fn update_active_css(&self) {
        let active = self.episode_filter();
        if active.is_default() {
            self.imp().is_active_circle.set_visible(false);
        } else {
            self.imp().is_active_circle.set_visible(true);
        }
    }

    pub fn connect_filter_changed<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("filter-changed", false, move |values| {
            let obj: Self = values[0].get().unwrap();
            f(&obj);
            None
        })
    }

    pub fn episode_filter(&self) -> EpisodeFilter {
        let order_str: String = self.get_action_state("order");
        let reverse_order = matches!(order_str.as_str(), "reversed");

        let played_str: String = self.get_action_state("played");
        let played = match played_str.as_str() {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        };

        let downloaded_str: String = self.get_action_state("downloaded");
        let downloaded = match downloaded_str.as_str() {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        };

        EpisodeFilter {
            reverse_order,
            downloaded,
            played,
            search: None,
        }
    }

    pub fn show_filter(&self) -> ShowFilter {
        let ep_filter = self.episode_filter();
        ShowFilter {
            reverse_order: ep_filter.reverse_order,
            any_downloaded: ep_filter.downloaded,
            completed: ep_filter.played,
            title_or_description: ep_filter.search,
        }
    }

    pub fn search_button(&self) -> &gtk::ToggleButton {
        &self.imp().search_button
    }

    fn get_action_state(&self, key: &'static str) -> String {
        self.imp()
            .group
            .borrow()
            .lookup_action(key)
            .unwrap()
            .state()
            .unwrap()
            .get()
            .unwrap()
    }
}

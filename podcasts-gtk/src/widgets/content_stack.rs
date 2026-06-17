// content_stack.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
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

use adw::glib::prelude::*;
use adw::prelude::*;
use anyhow::Result;
use async_channel::Sender;
use gettextrs::gettext;
use glib::clone;
use gtk::glib;
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use crate::app::Action;
use crate::utils::get_ignored_shows;
use crate::widgets::{EmptyView, FilterMenu, HomeView, ShowsView};
use podcasts_data::EpisodeWidgetModel;
use podcasts_data::dbqueries::is_episodes_populated;

#[derive(Debug, Clone)]
pub(crate) struct Content {
    overlay: gtk::Overlay,
    sender: Sender<Action>,
    progress_bar: gtk::ProgressBar,
    stack: adw::ViewStack,
    shows_bin: adw::Bin,
    shows: OnceCell<ShowsView>,
    home: HomeView,
    empty: EmptyView,
    filter_menu_stack: RefCell<adw::ViewStack>,
}

impl Content {
    pub(crate) fn new(
        sender: Sender<Action>,
        filter_menu_home: FilterMenu,
        filter_menu_shows: FilterMenu,
        filter_menu_stack: adw::ViewStack,
    ) -> Rc<Self> {
        let stack = adw::ViewStack::new();
        let shows_bin = adw::Bin::new();
        let shows = OnceCell::new();
        let home = HomeView::new(sender.clone(), filter_menu_home.clone());
        let overlay = gtk::Overlay::new();
        let empty = EmptyView::default();
        let progress_bar = gtk::ProgressBar::builder()
            .valign(gtk::Align::Start)
            .halign(gtk::Align::Fill)
            .visible(false)
            .tooltip_text(gettext("Fetching feeds…"))
            .build();
        progress_bar.add_css_class("osd");

        overlay.set_child(Some(&stack));
        overlay.add_overlay(&progress_bar);

        let home_page = stack.add_titled(&home, Some("home"), &gettext("New"));
        let shows_page = stack.add_titled(&shows_bin, Some("shows"), &gettext("Shows"));
        stack.add_named(&empty, Some("empty"));

        home_page.set_icon_name(Some("document-open-recent-symbolic"));
        shows_page.set_icon_name(Some("audio-input-microphone-symbolic"));

        let this = Rc::new(Self {
            overlay,
            sender,
            progress_bar,
            stack: stack.clone(),
            shows_bin,
            home,
            shows,
            empty,
            filter_menu_stack: RefCell::new(filter_menu_stack),
        });

        stack.connect_visible_child_notify(clone!(
            #[weak]
            filter_menu_shows,
            #[weak]
            this,
            move |s| {
                if let Some(name) = s.visible_child_name() {
                    if name == "shows" {
                        this.filter_menu_stack
                            .borrow()
                            .set_visible_child_name("shows");
                        this.init_shows(filter_menu_shows);
                    }
                    if name == "home" {
                        this.filter_menu_stack
                            .borrow()
                            .set_visible_child_name("home");
                    }
                }
            }
        ));

        if let Err(e) = this.check_empty_state() {
            error!("Failed to check for empty db state {e}");
        }

        this
    }

    pub(crate) fn update(&self) {
        self.update_home();
        self.update_shows();
        if let Err(e) = self.check_empty_state() {
            error!("Failed to check for empty db state {e}");
        }
    }

    pub(crate) fn update_home(&self) {
        if let Ok(home) = self.home.clone().downcast::<HomeView>() {
            home.reload(self.sender.clone())
        }
    }

    pub(crate) fn update_home_episode(&self, ep: &EpisodeWidgetModel) {
        if let Ok(home) = self.home.clone().downcast::<HomeView>() {
            home.update_episode(ep);
        }
    }

    pub(crate) fn update_shows(&self) {
        if let Some(shows) = self.shows.get() {
            shows.update_model();
        }
    }

    pub(crate) fn progress_bar(&self) -> &gtk::ProgressBar {
        &self.progress_bar
    }

    pub(crate) fn stack(&self) -> &adw::ViewStack {
        &self.stack
    }

    pub(crate) fn overlay(&self) -> &gtk::Overlay {
        &self.overlay
    }

    fn init_shows(&self, filter_menu: FilterMenu) {
        if self.shows.get().is_none() {
            self.shows
                .set({
                    info!("Init Shows View");
                    let new_shows = ShowsView::new(self.sender.clone(), filter_menu);
                    self.shows_bin.set_child(Some(&new_shows));
                    new_shows
                })
                .unwrap();
        }
    }

    pub(crate) fn go_to_home(&self) {
        if !self.is_in_empty_view() {
            self.stack.set_visible_child_name("home");
        }
    }

    pub(crate) fn go_to_shows(&self) {
        if !self.is_in_empty_view() {
            self.stack.set_visible_child_name("shows");
        }
    }

    pub(crate) fn switch_to_empty_views(&self) {
        self.stack.set_visible_child(&self.empty);
    }

    pub(crate) fn switch_to_populated(&self) {
        self.stack.set_visible_child(&self.home);
    }

    pub(crate) fn is_in_empty_view(&self) -> bool {
        self.stack
            .visible_child_name()
            .is_some_and(|name| name == "empty")
    }

    pub fn check_empty_state(&self) -> Result<()> {
        let ign = get_ignored_shows()?;
        debug!("IGNORED SHOWS {:?}", ign);
        if is_episodes_populated(&ign)? {
            send_blocking!(self.sender, Action::PopulatedState)
        } else {
            send_blocking!(self.sender, Action::EmptyState)
        };
        Ok(())
    }

    pub(crate) fn open_search(&self) {
        if let Some(name) = self.stack.visible_child_name() {
            if name == "shows" {
                self.shows.get().unwrap().open_search();
            } else if name == "home" {
                self.home.open_search();
            }
        }
    }
}

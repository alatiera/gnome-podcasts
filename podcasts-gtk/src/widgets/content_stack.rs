// content_stack.rs
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
use anyhow::Result;
use async_channel::Sender;
use std::cell::OnceCell;
use std::rc::Rc;

use crate::app::Action;
use crate::i18n::i18n;
use crate::utils::get_ignored_shows;
use crate::widgets::{EmptyView, HomeView, ShowsView};
use podcasts_data::dbqueries::is_episodes_populated;

#[derive(Debug, Clone)]
pub(crate) struct Content {
    overlay: gtk::Overlay,
    sender: Sender<Action>,
    progress_bar: gtk::ProgressBar,
    stack: adw::ViewStack,
    shows_bin: adw::Bin,
    shows: OnceCell<ShowsView>,
    // TODO drop the home_bin and just update the model
    //      of HomeView once ported to ListView
    home_bin: adw::Bin,
    empty: EmptyView,
}

impl Content {
    pub(crate) fn new(sender: Sender<Action>) -> Rc<Self> {
        let stack = adw::ViewStack::new();
        let shows_bin = adw::Bin::new();
        let shows = OnceCell::new();
        let home_bin = adw::Bin::new();
        let home = HomeView::new(sender.clone());
        let overlay = gtk::Overlay::new();
        let empty = EmptyView::default();
        let progress_bar = gtk::ProgressBar::builder()
            .valign(gtk::Align::Start)
            .halign(gtk::Align::Center)
            .visible(false)
            .build();
        progress_bar.add_css_class("osd");

        overlay.set_child(Some(&stack));
        overlay.add_overlay(&progress_bar);

        let home_page = stack.add_titled(&home_bin, Some("home"), &i18n("New"));
        let shows_page = stack.add_titled(&shows_bin, Some("shows"), &i18n("Shows"));
        stack.add_named(&empty, Some("empty"));

        home_page.set_icon_name(Some("document-open-recent-symbolic"));
        shows_page.set_icon_name(Some("audio-input-microphone-symbolic"));

        home_bin.set_child(Some(&home));

        let this = Rc::new(Self {
            overlay,
            sender,
            progress_bar,
            stack: stack.clone(),
            shows_bin,
            shows,
            home_bin,
            empty,
        });

        let weak = Rc::downgrade(&this);
        stack.connect_visible_child_notify(move |s| {
            if let Some(name) = s.visible_child_name() {
                if name == "shows" {
                    if let Some(this) = weak.upgrade() {
                        this.init_shows();
                    }
                }
            }
        });

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
        let home = HomeView::new(self.sender.clone());
        self.home_bin.set_child(Some(&home));
    }

    pub(crate) fn update_home_if_background(&self) {
        if self.stack.visible_child_name() != Some("home".into()) {
            self.update_home();
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

    fn init_shows(&self) {
        if self.shows.get().is_none() {
            self.shows
                .set({
                    info!("Init Shows View");
                    let new_shows = ShowsView::new(self.sender.clone());
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
        self.stack.set_visible_child(&self.home_bin);
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
}

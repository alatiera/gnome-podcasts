// populated.rs
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

use gtk::glib;
use gtk::prelude::*;
use gtk::StackTransitionType;

use anyhow::Result;
use glib::Sender;

use podcasts_data::dbqueries;
use podcasts_data::Show;

use crate::app::Action;
use crate::widgets::{ShowWidget, ShowsView};

use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub(crate) enum PopulatedState {
    View,
    Widget,
}

#[derive(Debug, Clone)]
pub(crate) struct PopulatedStack {
    container: gtk::Box,
    populated: ShowsView,
    show: ShowWidget,
    stack: gtk::Stack,
    state: PopulatedState,
    sender: Sender<Action>,
}

impl PopulatedStack {
    pub(crate) fn new(sender: Sender<Action>) -> PopulatedStack {
        let stack = gtk::Stack::new();
        let state = PopulatedState::View;
        let populated = ShowsView::new();
        let show = ShowWidget::default();
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        stack.add_named(populated.view(), Some("shows"));
        stack.add_named(&show, Some("widget"));
        container.append(&stack);

        PopulatedStack {
            container,
            stack,
            populated,
            show,
            state,
            sender,
        }
    }

    pub(crate) fn update(&mut self) {
        self.update_widget().map_err(|err| format!("{}", err)).ok();
        self.update_shows().map_err(|err| format!("{}", err)).ok();
    }

    pub(crate) fn update_shows(&mut self) -> Result<()> {
        // The current visible child might change depending on
        // removal and insertion in the gtk::Stack, so we have
        // to make sure it will stay the same.
        let s = self.state;
        self.replace_shows()?;
        self.switch_visible(s, StackTransitionType::Crossfade);

        Ok(())
    }

    pub(crate) fn replace_shows(&mut self) -> Result<()> {
        let old = &self.populated.view().clone();
        debug!("Name: {:?}", old.widget_name());

        let pop = ShowsView::new();
        self.populated = pop;
        self.stack.remove(old);
        self.stack.add_named(self.populated.view(), Some("shows"));

        Ok(())
    }

    pub(crate) fn replace_widget(&mut self, pd: Arc<Show>) -> Result<()> {
        let old = self.show.clone();

        let new = ShowWidget::new(pd, self.sender.clone());

        self.show = new;
        self.stack.remove(&old);
        self.stack.add_named(&self.show, Some("widget"));

        // The current visible child might change depending on
        // removal and insertion in the gtk::Stack, so we have
        // to make sure it will stay the same.
        let s = self.state;
        self.switch_visible(s, StackTransitionType::None);

        Ok(())
    }

    pub(crate) fn update_widget(&mut self) -> Result<()> {
        let id = self.show.show_id();
        if id.is_none() {
            return Ok(());
        }

        let pd = dbqueries::get_podcast_from_id(id.unwrap_or_default())?;
        self.replace_widget(Arc::new(pd))?;

        // The current visible child might change depending on
        // removal and insertion in the gtk::Stack, so we have
        // to make sure it will stay the same.
        let s = self.state;
        self.switch_visible(s, StackTransitionType::Crossfade);

        Ok(())
    }

    // Only update widget if its show_id is equal to pid.
    pub(crate) fn update_widget_if_same(&mut self, pid: i32) -> Result<()> {
        if self.show.show_id() != Some(pid) {
            debug!("Different widget. Early return");
            return Ok(());
        }

        self.update_widget()
    }

    pub(crate) fn container(&self) -> gtk::Box {
        self.container.clone()
    }

    pub(crate) fn switch_visible(&mut self, state: PopulatedState, animation: StackTransitionType) {
        use self::PopulatedState::*;

        match state {
            View => {
                self.stack.set_visible_child_full("shows", animation);
                self.state = View;
            }
            Widget => {
                self.stack.set_visible_child_full("widget", animation);
                self.state = Widget;
            }
        }
    }

    pub(crate) fn populated_state(&self) -> PopulatedState {
        self.state
    }
}

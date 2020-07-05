// home.rs
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

use gtk;
use gtk::prelude::*;
use gtk::StackTransitionType;

use anyhow::Result;
use crossbeam_channel::Sender;

use crate::app::Action;
use crate::stacks::State;
use crate::widgets::{EmptyView, HomeView};

use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) struct HomeStack {
    empty: EmptyView,
    episodes: Rc<HomeView>,
    stack: gtk::Stack,
    state: State,
    sender: Sender<Action>,
}

impl HomeStack {
    pub(crate) fn new(sender: Sender<Action>) -> Result<HomeStack> {
        let episodes = HomeView::new(sender.clone(), None)?;
        let empty = EmptyView::default();
        let stack = gtk::Stack::new();
        let state = State::Empty;

        stack.add_named(episodes.view.container(), "home");
        stack.add_named(empty.deref(), "empty");

        let home = HomeStack {
            empty,
            episodes,
            stack,
            state,
            sender,
        };

        Ok(home)
    }

    pub(crate) fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub(crate) fn update(&mut self) -> Result<()> {
        // Get the container of the view
        let old = &self.episodes.view.container().clone();

        // Copy the vertical scrollbar adjustment from the old view.
        let vadj = self.episodes.view.get_vadjustment();
        let eps = HomeView::new(self.sender.clone(), vadj)?;

        // Remove the old widget and add the new one
        // during this the previous view is removed,
        // and the visible child falls back to empty view.
        self.stack.remove(old);
        self.stack.add_named(eps.view.container(), "home");
        // Keep the previous state.
        let s = self.state;
        // Set the visible child back to the previous one to avoid
        // the stack transition animation to show the empty view
        self.switch_visible(s, StackTransitionType::None);

        // replace view in the struct too
        self.episodes = eps;

        // This might not be needed
        old.destroy();

        Ok(())
    }

    pub(crate) fn switch_visible(&mut self, s: State, animation: StackTransitionType) {
        use self::State::*;

        match s {
            Populated => {
                self.stack.set_visible_child_full("home", animation);
                self.state = Populated;
            }
            Empty => {
                self.stack.set_visible_child_full("empty", animation);
                self.state = Empty;
            }
        }
    }
}

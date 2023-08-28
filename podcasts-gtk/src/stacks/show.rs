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

use anyhow::Result;
use glib::Sender;
use gtk::glib;
use podcasts_data::dbqueries::is_episodes_populated;

use crate::app::Action;
use crate::stacks::content::State;
use crate::stacks::PopulatedStack;
use crate::utils::get_ignored_shows;
use crate::widgets::EmptyView;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) struct ShowStack {
    _empty: EmptyView,
    populated: Rc<RefCell<PopulatedStack>>,
    stack: gtk::Stack,
    state: State,
    sender: Sender<Action>,
}

impl ShowStack {
    pub(crate) fn new(sender: Sender<Action>) -> Self {
        let populated = Rc::new(RefCell::new(PopulatedStack::new(sender.clone())));
        let empty = EmptyView::default();
        let stack = gtk::Stack::new();
        let state = State::Empty;

        stack.add_named(&populated.borrow().container(), Some("populated"));
        stack.add_named(&empty, Some("empty"));

        let mut show = ShowStack {
            _empty: empty,
            populated,
            stack,
            state,
            sender,
        };

        let res = show.determine_state();
        debug_assert!(res.is_ok());
        show
    }

    pub(crate) fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub(crate) fn populated(&self) -> Rc<RefCell<PopulatedStack>> {
        self.populated.clone()
    }

    pub(crate) fn update(&mut self) -> Result<()> {
        self.populated.borrow_mut().update();
        self.determine_state()
    }

    pub(crate) fn switch_visible(&mut self, s: State) {
        use self::State::*;

        match s {
            Populated => {
                self.stack.set_visible_child_name("populated");
                self.state = Populated;
            }
            Empty => {
                self.stack.set_visible_child_name("empty");
                self.state = Empty;
            }
        };
    }

    fn determine_state(&mut self) -> Result<()> {
        let ign = get_ignored_shows()?;
        debug!("IGNORED SHOWS {:?}", ign);
        if is_episodes_populated(&ign)? {
            send!(self.sender, Action::PopulatedState)
        } else {
            send!(self.sender, Action::EmptyState)
        };

        Ok(())
    }
}

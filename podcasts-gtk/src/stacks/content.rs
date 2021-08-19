// content.rs
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

use gtk::prelude::*;

use anyhow::Result;
use glib::Sender;

use crate::app::Action;
use crate::stacks::{HomeStack, ShowStack};

use std::cell::RefCell;
use std::rc::Rc;

use crate::i18n::i18n;

#[derive(Debug, Clone, Copy)]
pub(crate) enum State {
    Populated,
    Empty,
}

#[derive(Debug, Clone)]
pub(crate) struct Content {
    container: gtk::Box,
    stack: gtk::Stack,
    shows: Rc<RefCell<ShowStack>>,
    home: Rc<RefCell<HomeStack>>,
    sender: Sender<Action>,
}

impl Content {
    pub(crate) fn new(sender: &Sender<Action>) -> Result<Rc<Content>> {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let stack = gtk::Stack::new();
        let home = Rc::new(RefCell::new(HomeStack::new(sender.clone())?));
        let shows = Rc::new(RefCell::new(ShowStack::new(sender.clone())));

        // container will hold the header bar and the content
        container.set_widget_name("content");
        container.pack_end(&stack, true, true, 0);
        stack.add_titled(&home.borrow().get_stack(), "home", &i18n("New"));
        stack.add_titled(&shows.borrow().get_stack(), "shows", &i18n("Shows"));

        stack.set_child_icon_name(
            &home.borrow().get_stack(),
            Some("document-open-recent-symbolic"),
        );
        stack.set_child_icon_name(
            &shows.borrow().get_stack(),
            Some("audio-input-microphone-symbolic"),
        );

        let con = Content {
            container,
            stack,
            shows,
            home,
            sender: sender.clone(),
        };
        Ok(Rc::new(con))
    }

    pub(crate) fn update(&self) {
        self.update_home();
        self.update_shows();
    }

    pub(crate) fn update_home(&self) {
        self.home
            .borrow_mut()
            .update()
            .map_err(|err| error!("Failed to update HomeView: {}", err))
            .ok();
    }

    pub(crate) fn update_home_if_background(&self) {
        if self.stack.visible_child_name() != Some("home".into()) {
            self.update_home();
        }
    }

    fn update_shows(&self) {
        self.shows
            .borrow_mut()
            .update()
            .map_err(|err| error!("Failed to update ShowsView: {}", err))
            .ok();
    }

    pub(crate) fn update_shows_view(&self) {
        self.shows
            .borrow_mut()
            .update()
            .map_err(|err| error!("Failed to update ShowsView: {}", err))
            .ok();
    }

    pub(crate) fn update_widget_if_same(&self, pid: i32) {
        let pop = self.shows.borrow().populated();
        pop.borrow_mut()
            .update_widget_if_same(pid)
            .map_err(|err| error!("Failed to update ShowsWidget: {}", err))
            .ok();
    }

    pub(crate) fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }
    pub(crate) fn get_container(&self) -> gtk::Box {
        self.container.clone()
    }

    pub(crate) fn get_shows(&self) -> Rc<RefCell<ShowStack>> {
        self.shows.clone()
    }

    pub(crate) fn switch_to_empty_views(&self) {
        use gtk::StackTransitionType::*;

        self.home
            .borrow_mut()
            .switch_visible(State::Empty, Crossfade);
        self.shows.borrow_mut().switch_visible(State::Empty);
    }

    pub(crate) fn switch_to_populated(&self) {
        use gtk::StackTransitionType::*;

        self.home
            .borrow_mut()
            .switch_visible(State::Populated, Crossfade);
        self.shows.borrow_mut().switch_visible(State::Populated);
    }
}

// episode_menu.rs
//
// Copyright 2021 nee <nee-git@patchouli.garden>
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

use async_channel::Sender;
use glib::clone;
use gtk::prelude::*;
use gtk::{gio, glib};
use std::sync::Arc;

use crate::app::Action;
use podcasts_data::Show;
use podcasts_data::{EpisodeId, EpisodeModel};

#[derive(Debug, Clone)]
pub(crate) struct EpisodeMenu {
    pub(crate) menu: gio::MenuModel,
    go_to_show: gio::SimpleAction,
    copy_episode_url: gio::SimpleAction,
    mark_as_played: gio::SimpleAction,
    mark_as_unplayed: gio::SimpleAction,
    pub(crate) group: gio::SimpleActionGroup,
}

impl Default for EpisodeMenu {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/episode_menu.ui");
        let menu = builder.object("episode_menu").unwrap();
        let go_to_show = gio::SimpleAction::new("go-to-show", None);
        let copy_episode_url = gio::SimpleAction::new("copy-episode-url", None);
        let mark_as_played = gio::SimpleAction::new("mark-as-played", None);
        let mark_as_unplayed = gio::SimpleAction::new("mark-as-unplayed", None);
        let group = gio::SimpleActionGroup::new();

        EpisodeMenu {
            menu,
            go_to_show,
            copy_episode_url,
            mark_as_played,
            mark_as_unplayed,
            group,
        }
    }
}

impl EpisodeMenu {
    pub fn new(sender: &Sender<Action>, ep: &dyn EpisodeModel, show: Arc<Show>) -> Self {
        let s = Self::default();
        s.init(sender, ep, show);
        s
    }

    fn init(&self, sender: &Sender<Action>, ep: &dyn EpisodeModel, show: Arc<Show>) {
        self.connect_go_to_show(sender, show);
        self.connect_mark_as_played(sender, ep.id());
        self.update_played_state(ep);
        self.connect_copy_episode_url(sender, ep);
    }

    fn update_played_state(&self, ep: &dyn EpisodeModel) {
        let played = ep.played();
        self.mark_as_played.set_enabled(played.is_none());
        self.mark_as_unplayed.set_enabled(played.is_some());
    }

    fn connect_go_to_show(&self, sender: &Sender<Action>, show: Arc<Show>) {
        self.go_to_show.connect_activate(clone!(
            #[strong]
            sender,
            #[strong]
            show,
            move |_, _| {
                send_blocking!(sender, Action::GoToShow(show.clone()));
            }
        ));
        self.group.add_action(&self.go_to_show);
    }

    fn connect_copy_episode_url(&self, sender: &Sender<Action>, ep: &dyn EpisodeModel) {
        let ep_id = ep.id();
        if ep.uri().is_some() {
            self.copy_episode_url.connect_activate(clone!(
                #[strong]
                sender,
                move |_, _| {
                    send_blocking!(sender, Action::CopyUrl(ep_id));
                }
            ));
            self.group.add_action(&self.copy_episode_url);
        }
    }

    fn connect_mark_as_played(&self, sender: &Sender<Action>, ep_id: EpisodeId) {
        self.mark_as_played.connect_activate(clone!(
            #[strong]
            sender,
            move |_, _| {
                send_blocking!(sender, Action::MarkAsPlayed(true, ep_id));
            }
        ));
        self.group.add_action(&self.mark_as_played);
        self.mark_as_unplayed.connect_activate(clone!(
            #[strong]
            sender,
            move |_, _| {
                send_blocking!(sender, Action::MarkAsPlayed(false, ep_id));
            }
        ));
        self.group.add_action(&self.mark_as_unplayed);
    }
}

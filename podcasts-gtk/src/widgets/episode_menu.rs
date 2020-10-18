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

use gio::prelude::ActionMapExt;
use glib::clone;
use gtk::prelude::*;

use glib::Sender;

use podcasts_data::Episode;
use podcasts_data::Show;

use crate::app::Action;

use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct EpisodeMenu {
    pub(crate) menu: gio::MenuModel,
    go_to_show: gio::SimpleAction,
    copy_episode_url: gio::SimpleAction,
    group: gio::SimpleActionGroup,
}

impl Default for EpisodeMenu {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/episode_menu.ui");
        let menu = builder.object("episode_menu").unwrap();
        let go_to_show = gio::SimpleAction::new("go-to-show", None);
        let copy_episode_url = gio::SimpleAction::new("copy-episode-url", None);
        let group = gio::SimpleActionGroup::new();

        EpisodeMenu {
            menu,
            go_to_show,
            copy_episode_url,
            group,
        }
    }
}

impl EpisodeMenu {
    pub fn new(sender: &Sender<Action>, ep: Arc<Episode>, show: Arc<Show>) -> Self {
        let s = Self::default();
        s.init(sender, ep, show);
        s
    }

    fn init(&self, sender: &Sender<Action>, ep: Arc<Episode>, show: Arc<Show>) {
        self.connect_go_to_show(sender, show);
        self.connect_copy_episode_url(sender, ep);

        let app = gio::Application::default()
            .expect("Could not get default application")
            .downcast::<gtk::Application>()
            .unwrap();
        let win = app.active_window().expect("No active window");
        win.insert_action_group("episode", Some(&self.group));
    }

    fn connect_go_to_show(&self, sender: &Sender<Action>, show: Arc<Show>) {
        self.go_to_show
            .connect_activate(clone!(@strong sender, @strong show => move |_,_| {
                send!(sender, Action::HeaderBarShowTile(show.title().into()));
                send!(sender, Action::ReplaceWidget(show.clone()));
                send!(sender, Action::ShowWidgetAnimated);
            }));
        self.group.add_action(&self.go_to_show);
    }

    fn connect_copy_episode_url(&self, sender: &Sender<Action>, ep: Arc<Episode>) {
        if let Some(uri) = ep.uri().map(|s| s.to_string()) {
            self.copy_episode_url
                .connect_activate(clone!(@strong sender => move |_,_| {
                    copy_text(&uri);
                    send!(sender, Action::CopiedUrlNotification);
                }));
            self.group.add_action(&self.copy_episode_url);
        }
    }
}

fn copy_text(text: &str) -> Option<()> {
    let display = gtk::gdk::Display::default()?;
    let clipboard = display.clipboard();
    clipboard.set_text(text);
    Some(())
}

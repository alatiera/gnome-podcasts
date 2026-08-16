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

use crate::app::Action;
use crate::widgets::episode::on_download_clicked;
use podcasts_data::{EpisodeId, EpisodeModel};
use podcasts_data::{ShowId, dbqueries};

#[derive(Debug, Clone)]
pub(crate) struct EpisodeMenu {
    pub(crate) menu: gio::MenuModel,
    go_to_show: gio::SimpleAction,
    copy_episode_url: gio::SimpleAction,
    mark_as_played: gio::SimpleAction,
    mark_as_unplayed: gio::SimpleAction,
    download: gio::SimpleAction,
    delete: gio::SimpleAction,
    move_up_in_queue: gio::SimpleAction,
    move_down_in_queue: gio::SimpleAction,
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
        let download = gio::SimpleAction::new("download", None);
        let delete = gio::SimpleAction::new("delete", None);
        let move_up_in_queue = gio::SimpleAction::new("move-up-in-queue", None);
        let move_down_in_queue = gio::SimpleAction::new("move-down-in-queue", None);
        let group = gio::SimpleActionGroup::new();

        EpisodeMenu {
            menu,
            go_to_show,
            copy_episode_url,
            mark_as_played,
            mark_as_unplayed,
            download,
            delete,
            move_up_in_queue,
            move_down_in_queue,
            group,
        }
    }
}

impl EpisodeMenu {
    pub fn new(
        sender: &Sender<Action>,
        ep: &dyn EpisodeModel,
        show: Option<ShowId>,
        is_queue_view: bool,
    ) -> Self {
        let s = Self::default();
        s.init(sender, ep, show, is_queue_view);
        s
    }

    fn init(
        &self,
        sender: &Sender<Action>,
        ep: &dyn EpisodeModel,
        show: Option<ShowId>,
        is_queue_view: bool,
    ) {
        if let Some(show_id) = show {
            self.connect_go_to_show(show_id);
        }
        self.connect_mark_as_played(sender, ep.id());
        self.update_played_state(ep);
        self.connect_copy_episode_url(sender, ep);
        let is_downloaded = ep.is_downloaded();
        if !is_downloaded {
            self.connect_download(sender, ep);
        } else {
            self.connect_delete(sender, ep);
            if is_queue_view {
                if matches!(dbqueries::is_last_position_in_queue(ep.id()), Ok(false)) {
                    self.connect_move_down_in_queue(sender, ep);
                }
                if matches!(dbqueries::is_first_position_in_queue(ep.id()), Ok(false)) {
                    self.connect_move_up_in_queue(sender, ep);
                }
            }
        }
    }

    fn update_played_state(&self, ep: &dyn EpisodeModel) {
        let played = ep.played();
        self.mark_as_played.set_enabled(played.is_none());
        self.mark_as_unplayed.set_enabled(played.is_some());
    }

    fn connect_go_to_show(&self, id: ShowId) {
        self.go_to_show.connect_activate(move |_, _| {
            if let Some(app) = gio::Application::default() {
                app.activate_action("go-to-show", Some(&id.0.into()));
            }
        });
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

    fn connect_download(&self, sender: &Sender<Action>, ep: &dyn EpisodeModel) {
        let ep_id = ep.id();
        if ep.uri().is_some() {
            self.download.connect_activate(clone!(
                #[strong]
                sender,
                move |_, _| {
                    if let Ok(ep) = dbqueries::get_episode_widget_from_id(ep_id) {
                        if let Err(e) = on_download_clicked(&ep, &sender) {
                            error!("Failed to start download: {e}");
                        }
                    } else {
                        error!("Failed to start download, no episode found with id: {ep_id:?}");
                    }
                }
            ));
            self.group.add_action(&self.download);
        }
    }

    fn connect_delete(&self, sender: &Sender<Action>, ep: &dyn EpisodeModel) {
        let ep_id = ep.id();
        if ep.uri().is_some() {
            self.delete.connect_activate(clone!(
                #[strong]
                sender,
                move |_, _| {
                    if let Ok(episode) = dbqueries::get_episode_from_id(ep_id) {
                        let mut cleaner_ep = podcasts_data::EpisodeCleanerModel::from(episode);
                        if let Err(e) = podcasts_data::utils::delete_local_content(&mut cleaner_ep)
                        {
                            error!("failed to delete ep {e}");
                        } else {
                            // Remove the episode from the queue
                            send_blocking!(sender, Action::RemoveFromQueue(ep_id));
                            send_blocking!(sender, Action::RefreshEpisode(ep_id));
                        }
                    }
                }
            ));
            self.group.add_action(&self.delete);
        }
    }

    fn connect_move_up_in_queue(&self, sender: &Sender<Action>, ep: &dyn EpisodeModel) {
        let ep_id = ep.id();
        if ep.uri().is_some() {
            self.move_up_in_queue.connect_activate(clone!(
                #[strong]
                sender,
                move |_, _| {
                    if let Ok(episode) = dbqueries::get_episode_widget_from_id(ep_id) {
                        // Move the episode up in the queue
                        send_blocking!(sender, Action::MoveUpInQueue(episode.id()));
                    }
                }
            ));
            self.group.add_action(&self.move_up_in_queue);
        }
    }

    fn connect_move_down_in_queue(&self, sender: &Sender<Action>, ep: &dyn EpisodeModel) {
        let ep_id = ep.id();
        if ep.uri().is_some() {
            self.move_down_in_queue.connect_activate(clone!(
                #[strong]
                sender,
                move |_, _| {
                    if let Ok(episode) = dbqueries::get_episode_widget_from_id(ep_id) {
                        // Move the episode down in the queue
                        send_blocking!(sender, Action::MoveDownInQueue(episode.id()));
                    }
                }
            ));
            self.group.add_action(&self.move_down_in_queue);
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

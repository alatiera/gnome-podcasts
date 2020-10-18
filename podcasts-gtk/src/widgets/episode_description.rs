// episode_description.rs
//
// Copyright 2020 nee <nee-git@patchouli.garden>
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

use glib::{clone, Sender};
use podcasts_data::{Episode, Show};

use crate::app::Action;
use crate::utils::{self};
use crate::widgets::appnotif::InAppNotification;
use crate::widgets::EpisodeMenu;

use crate::episode_description_parser;
use crate::i18n::i18n;
use chrono::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct EpisodeDescription {
    pub(crate) container: gtk::Box,
    back_button: gtk::Button,
    menu_button: gtk::MenuButton,
    image: gtk::Image,
    podcast_title: gtk::Label,
    title_label: gtk::Label,
    duration_date_label: gtk::Label,
    description_label: gtk::Label,
    episode_id: Option<i32>,
}

impl Default for EpisodeDescription {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/episode_description.ui");
        let container: gtk::Box = builder.object("episode_container").unwrap();
        let back_button: gtk::Button = builder.object("back_button").unwrap();
        let menu_button: gtk::MenuButton = builder.object("menu_button").unwrap();
        let image = builder.object("cover").unwrap();
        let podcast_title: gtk::Label = builder.object("podcast_title").unwrap();
        let title: gtk::Label = builder.object("episode_title").unwrap();
        let duration_date_label: gtk::Label = builder.object("episode_duration_date").unwrap();

        let label: gtk::Label = builder.object("episode_description").unwrap();

        EpisodeDescription {
            container,
            back_button,
            menu_button,
            image,
            podcast_title,
            title_label: title,
            duration_date_label,
            description_label: label,
            episode_id: None,
        }
    }
}

impl EpisodeDescription {
    pub(crate) fn new(
        ep: Arc<Episode>,
        show: Arc<Show>,
        sender: Sender<Action>,
    ) -> Rc<EpisodeDescription> {
        let mut episode_description = EpisodeDescription::default();

        episode_description.init(&ep, &show);

        let menu = EpisodeMenu::new(&sender, ep, show);
        episode_description
            .menu_button
            .set_menu_model(Some(&menu.menu));
        episode_description
            .back_button
            .connect_clicked(clone!(@strong sender => move |_| {
                send!(sender, Action::MoveBackOnDeck);
            }));

        Rc::new(episode_description)
    }

    fn init(&mut self, ep: &Episode, show: &Show) {
        self.episode_id = Some(ep.rowid());

        if let Some(t) = ep.description() {
            let default_text = self.description_label.text();

            let markup = episode_description_parser::html2pango_markup(t);
            self.description_label.set_markup(&markup);
            // recover from invalid markup
            if self.description_label.text() == default_text {
                self.description_label.set_text(t);
            }
        };
        let duration = ep.duration().map(|s| {
            let seconds = s % 60;
            let minutes = (s / 60) % 60;
            let hours = (s / 60) / 60;
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        });
        let now_utc = Utc::now();
        let ep_utc = Utc.timestamp(i64::from(ep.epoch()), 0);
        // If the episode is from a different year, print year as well
        let date = if now_utc.year() != ep_utc.year() {
            ep_utc.format("%e %b %Y").to_string()
        // Else omit the year from the label
        } else {
            ep_utc.format("%e %b").to_string()
        };

        let duration_date = match duration {
            Some(duration) => format!("{} · {}", duration, date),
            None => date,
        };

        self.title_label.set_text(ep.title());
        self.podcast_title.set_text(show.title());
        self.duration_date_label.set_text(&duration_date);
        self.set_cover(ep.show_id());
    }

    fn set_cover(&self, show_id: i32) {
        utils::set_image_from_path(&self.image, show_id, 64)
            .map_err(|err| error!("Failed to set a cover: {}", err))
            .ok();
    }

    pub(crate) fn copied_url_notif() -> InAppNotification {
        let callback = move |revealer: gtk::Revealer| {
            revealer.set_reveal_child(false);
            glib::Continue(false)
        };
        let text = i18n("Copied URL to clipboard!");
        let undo_callback: Option<fn()> = None;
        InAppNotification::new(&text, 2000, callback, undo_callback)
    }
}

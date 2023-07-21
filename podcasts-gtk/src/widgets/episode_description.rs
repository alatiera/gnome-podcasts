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

use gtk::glib;
use gtk::CompositeTemplate;

use glib::subclass::InitializingObject;
use glib::{clone, Sender};
use podcasts_data::{Episode, Show};

use crate::app::Action;
use crate::utils::{self};
use crate::widgets::EpisodeMenu;

use crate::episode_description_parser;
use adw::subclass::prelude::*;
use anyhow::Result;
use chrono::prelude::*;
use std::sync::Arc;

use tokio::sync::oneshot::error::TryRecvError;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::WidgetExt;
use podcasts_data::{dbqueries, downloader};

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/episode_description.ui")]
pub struct EpisodeDescriptionPriv {
    #[template_child]
    menu_button: TemplateChild<gtk::MenuButton>,
    #[template_child]
    cover: TemplateChild<gtk::Image>,
    #[template_child]
    podcast_title: TemplateChild<gtk::Label>,
    #[template_child]
    episode_title: TemplateChild<gtk::Label>,
    #[template_child]
    episode_duration: TemplateChild<gtk::Label>,
    #[template_child]
    description: TemplateChild<gtk::Label>,
    #[template_child]
    episode_specific_cover: TemplateChild<gtk::Picture>,
}

impl EpisodeDescriptionPriv {
    fn init(&self, sender: Sender<Action>, ep: Arc<Episode>, show: Arc<Show>) {
        self.set_description(&ep);
        self.set_duration(&ep);
        self.episode_title.set_text(ep.title());
        self.podcast_title.set_text(show.title());
        self.set_cover(ep.show_id());
        if let Some(uri) = ep.image_uri().as_ref() {
            // don't show if it's the same as the show cover
            if *uri != show.image_uri().unwrap_or("") {
                let _ = self.set_episode_specific_cover(ep.show_id(), uri);
            }
        }

        let id = ep.rowid();
        let menu = EpisodeMenu::new(&sender, ep, show);
        self.menu_button.set_menu_model(Some(&menu.menu));

        self.description.connect_activate_link(move |_, url| {
            if let Some(seconds_str) = url.strip_prefix("jump:") {
                if let Ok(seconds) = seconds_str.parse() {
                    send!(sender, Action::InitEpisodeAt(id, seconds));
                } else {
                    error!("failed to parse jump link: {}", url);
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }

    fn set_description(&self, ep: &Episode) {
        if let Some(t) = ep.description() {
            let default_text = self.description.text();

            let markup = episode_description_parser::html2pango_markup(t);
            self.description.set_markup(&markup);
            // recover from invalid markup
            if self.description.text() == default_text {
                let plain = html2text::from_read(t.as_bytes(), t.as_bytes().len());
                self.description.set_text(&plain);
            }
        };
    }

    fn set_duration(&self, ep: &Episode) {
        let duration = ep.duration().map(|s| {
            let seconds = s % 60;
            let minutes = (s / 60) % 60;
            let hours = (s / 60) / 60;
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        });
        let now_utc = Utc::now();
        let ep_utc = Utc.timestamp_opt(i64::from(ep.epoch()), 0).unwrap();
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
        self.episode_duration.set_text(&duration_date);
    }

    fn set_cover(&self, show_id: i32) {
        utils::set_image_from_path(&self.cover, show_id, 64)
            .map_err(|err| error!("Failed to set a cover: {}", err))
            .ok();
    }

    fn set_episode_specific_cover(&self, show_id: i32, uri: &str) -> Result<()> {
        let pd = dbqueries::get_podcast_cover_from_id(show_id)?;
        let widget = self;

        let (sender, mut receiver) = tokio::sync::oneshot::channel();
        let callback = clone!(@weak widget =>  @default-return glib::ControlFlow::Break, move || {
            match receiver.try_recv() {
                Err(TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(TryRecvError::Closed) => glib::ControlFlow::Break,
                Ok(Ok(path)) => {
                    if let Ok(px) = Pixbuf::from_file(path)
                    {
                        let texture = gtk::gdk::Texture::for_pixbuf(&px);
                        widget.episode_specific_cover.set_paintable(Some(&texture));
                        widget.episode_specific_cover.set_visible(true);
                    }
                    glib::ControlFlow::Break
                },
                _ => glib::ControlFlow::Break
            }
        });

        let uri = uri.to_owned();
        crate::RUNTIME.spawn(clone!(@strong pd => async move {
            let path = downloader::cache_episode_image(&pd, &uri, true).await;
            sender.send(path).expect("episode_specific_image: channel was dropped unexpectedly")
        }));

        glib::timeout_add_local(std::time::Duration::from_millis(25), callback);
        Ok(())
    }
}

#[glib::object_subclass]
impl ObjectSubclass for EpisodeDescriptionPriv {
    const NAME: &'static str = "PdEpisodeDescription";
    type Type = EpisodeDescription;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for EpisodeDescriptionPriv {}
impl ObjectImpl for EpisodeDescriptionPriv {}
impl NavigationPageImpl for EpisodeDescriptionPriv {
    fn shown(&self) {
        self.description.set_selectable(true);
    }
}

glib::wrapper! {
    pub struct EpisodeDescription(ObjectSubclass<EpisodeDescriptionPriv>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl EpisodeDescription {
    pub(crate) fn new(ep: Arc<Episode>, show: Arc<Show>, sender: Sender<Action>) -> Self {
        let widget: Self = glib::Object::new();
        widget.imp().init(sender, ep, show);
        widget
    }
}

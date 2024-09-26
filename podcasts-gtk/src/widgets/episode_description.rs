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

use adw::subclass::prelude::*;
use anyhow::Result;
use async_channel::Sender;
use chrono::prelude::*;
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use std::borrow::Borrow;
use std::sync::Arc;

use crate::app::Action;
use crate::episode_description_parser;
use crate::widgets::DownloadProgressBar;
use crate::widgets::EpisodeMenu;
use podcasts_data::EpisodeWidgetModel;
use podcasts_data::{dbqueries, downloader};
use podcasts_data::{Episode, EpisodeId, EpisodeModel, Show, ShowId};

pub enum EpisodeDescriptionAction {
    EpisodeSpecificImage(gtk::gdk::Texture),
}

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

    #[template_child]
    progressbar: TemplateChild<DownloadProgressBar>,

    #[template_child]
    stream_button: TemplateChild<gtk::Button>,
    #[template_child]
    download_button: TemplateChild<gtk::Button>,
    #[template_child]
    cancel_button: TemplateChild<gtk::Button>,
    #[template_child]
    play_button: TemplateChild<gtk::Button>,
    #[template_child]
    delete_button: TemplateChild<gtk::Button>,
}

impl EpisodeDescriptionPriv {
    fn init(&self, sender: Sender<Action>, ep: Arc<Episode>, show: Arc<Show>) {
        let (ed_sender, r) = async_channel::unbounded();
        crate::MAINCONTEXT.spawn_local(clone!(
            #[weak(rename_to = this)]
            self,
            async move {
                while let Ok(action) = r.recv().await {
                    this.do_action(action);
                }
            }
        ));

        self.set_description(&ep);
        self.set_duration(&ep);
        self.episode_title.set_text(ep.title());
        self.podcast_title.set_text(show.title());
        self.set_cover(ep.show_id());
        if let Some(uri) = ep.image_uri().as_ref() {
            // don't show if it's the same as the show cover
            if *uri != show.image_uri().unwrap_or("") {
                let _ = self.set_episode_specific_cover(ed_sender, ep.show_id(), uri);
            }
        }

        let id = ep.id();
        self.description.connect_activate_link(clone!(
            #[strong]
            sender,
            move |_, url| {
                if let Some(seconds_str) = url.strip_prefix("jump:") {
                    if let Ok(seconds) = seconds_str.parse() {
                        send_blocking!(sender, Action::InitEpisodeAt(id, seconds));
                    } else {
                        error!("failed to parse jump link: {}", url);
                    }
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));

        let ep: &Episode = ep.borrow();
        if ep.uri().is_some() {
            self.init_buttons(sender, ep, id);
            self.determine_button_state(&ep.clone().into());
        }
    }

    fn init_buttons(&self, sender: Sender<Action>, ep: &Episode, id: EpisodeId) {
        self.stream_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                send_blocking!(sender, Action::StreamEpisode(id));
            }
        ));

        self.play_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                send_blocking!(sender, Action::InitEpisode(id));
            }
        ));

        let show_id = ep.show_id();
        self.download_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[strong]
            sender,
            move |_| {
                use podcasts_data::utils::get_download_dir;
                if let Err(e) = (|| {
                    let pd = dbqueries::get_podcast_from_id(show_id)?;
                    let download_dir = get_download_dir(pd.title())?;
                    crate::manager::add(sender.clone(), id, download_dir)?;
                    Ok::<(), anyhow::Error>(())
                })() {
                    error!("failed to start download {e}");
                }
                this.refresh_buttons(id);
                this.progressbar.grab_focus();
                send_blocking!(sender, Action::RefreshEpisodesView);
                send_blocking!(sender, Action::RefreshWidgetIfSame(show_id));
            }
        ));

        self.delete_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            #[strong]
            sender,
            move |_| {
                if let Ok(ep) = dbqueries::get_episode_from_id(id) {
                    let mut cleaner_ep = podcasts_data::EpisodeCleanerModel::from(ep);
                    if let Err(e) = podcasts_data::utils::delete_local_content(&mut cleaner_ep) {
                        error!("failed to delete ep {e}");
                    }
                }
                this.refresh_buttons(id);
                send_blocking!(sender, Action::RefreshEpisodesView);
                send_blocking!(sender, Action::RefreshWidgetIfSame(show_id));
            }
        ));

        self.progressbar.init(ep.id());
        self.progressbar.connect_state_change(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                this.refresh_buttons(id);
            }
        ));
        self.cancel_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                if let Err(e) = this.progressbar.cancel() {
                    error!("failed to cancel download {e}");
                }
            }
        ));
    }

    fn refresh_buttons(&self, id: EpisodeId) {
        match dbqueries::get_episode_widget_from_id(id) {
            Ok(ep) => self.determine_button_state(&ep),
            Err(e) => error!("failed to fetch episode for description refresh {e}"),
        }
    }

    fn determine_button_state(&self, ep: &EpisodeWidgetModel) {
        let is_downloading = self.progressbar.check_if_downloading().unwrap_or(false);
        self.cancel_button.set_visible(is_downloading);
        let is_downloaded = ep.local_uri().is_some();
        self.download_button
            .set_visible(!is_downloaded && !is_downloading);
        self.stream_button.set_visible(!is_downloaded);
        self.delete_button.set_visible(is_downloaded);
        self.play_button.set_visible(is_downloaded);
    }

    fn set_description(&self, ep: &Episode) {
        if let Some(t) = ep.description() {
            let default_text = self.description.text();

            let markup = episode_description_parser::html2pango_markup(t);
            self.description.set_markup(&markup);
            // recover from invalid markup
            if self.description.text() == default_text {
                let plain = html2text::config::plain()
                    .string_from_read(t.as_bytes(), t.as_bytes().len())
                    .unwrap_or_else(|_| t.to_string());
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
        let now = Local::now();
        let ep_local = DateTime::<Local>::from(ep.epoch().and_utc());
        // If the episode is from a different year, print year as well
        let date = if now.year() != ep_local.year() {
            ep_local
                .format_localized("%e %b %Y", *crate::CHRONO_LOCALE)
                .to_string()
            // Else omit the year from the label
        } else {
            ep_local
                .format_localized("%e %b", *crate::CHRONO_LOCALE)
                .to_string()
        };

        let duration_date = match duration {
            Some(duration) => format!("{} · {}", duration, date),
            None => date,
        };
        self.episode_duration.set_text(&duration_date);
    }

    fn set_cover(&self, show_id: ShowId) {
        crate::download_covers::load_widget_texture(&self.cover.get(), show_id, crate::Thumb64);
    }

    fn set_episode_specific_cover(
        &self,
        sender: Sender<EpisodeDescriptionAction>,
        show_id: ShowId,
        uri: &str,
    ) -> Result<()> {
        let pd = dbqueries::get_podcast_cover_from_id(show_id)?;
        let uri = uri.to_owned();
        crate::RUNTIME.spawn(clone!(
            #[strong]
            pd,
            async move {
                if let Err(e) = async move {
                    let path = downloader::cache_episode_image(&pd, &uri, true).await?;
                    let texture = gtk::gdk::Texture::from_filename(path)?;
                    send!(
                        sender,
                        EpisodeDescriptionAction::EpisodeSpecificImage(texture)
                    );
                    Ok::<(), anyhow::Error>(())
                }
                .await
                {
                    error!("failed to get episode specific cover: {e}");
                }
            }
        ));
        Ok(())
    }

    fn do_action(&self, action: EpisodeDescriptionAction) -> glib::ControlFlow {
        match action {
            EpisodeDescriptionAction::EpisodeSpecificImage(texture) => {
                self.episode_specific_cover.set_paintable(Some(&texture));
                self.episode_specific_cover.set_visible(true);
            }
        }
        glib::ControlFlow::Continue
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
        widget.update_episode_menu(&sender, ep.as_ref(), show.clone());
        widget.imp().init(sender, ep, show);

        widget
    }

    pub(crate) fn update_episode_menu(
        &self,
        sender: &Sender<Action>,
        ep: &dyn EpisodeModel,
        show: Arc<Show>,
    ) {
        let menu = EpisodeMenu::new(sender, ep, show);
        self.imp().menu_button.set_menu_model(Some(&menu.menu));
        self.insert_action_group("episode", Some(&menu.group));
    }
}

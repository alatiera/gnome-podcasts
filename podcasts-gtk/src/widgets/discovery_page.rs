// discovery_settings.rs
//
// Copyright 2022-2024 nee <nee-git@patchouli.garden>
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

use adw::prelude::*;
use adw::subclass::prelude::*;
use anyhow::Result;
use async_channel::Sender;
use gettextrs::gettext;
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::CompositeTemplate;
use gtk::glib;
use std::sync::Arc;
use url::Url;

use crate::app::Action;
use crate::utils::{itunes_to_rss, soundcloud_to_rss};
use podcasts_data::dbqueries;
use podcasts_data::discovery::SearchError::NoSearchPlatformsSelected;
use podcasts_data::discovery::{ALL_PLATFORM_IDS, SearchError, search};

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/discovery_page.ui")]
pub struct DiscoveryPagePriv {
    #[template_child]
    list: TemplateChild<adw::PreferencesGroup>,
    #[template_child]
    entry: TemplateChild<gtk::Entry>,
    #[template_child]
    search_button: TemplateChild<gtk::Button>,
    #[template_child]
    loading_spinner: TemplateChild<adw::Spinner>,
    #[template_child]
    no_platforms_selected_label: TemplateChild<gtk::Label>,
}

impl DiscoveryPagePriv {
    fn init(&self, sender: &Sender<Action>) {
        let (loading_done, receiver) = async_channel::bounded(1);
        crate::MAINCONTEXT.spawn_local(clone!(
            #[weak(rename_to = this)]
            self,
            async move {
                while let Ok(result) = receiver.recv().await {
                    if let Err(NoSearchPlatformsSelected) = result {
                        this.entry.add_css_class("error");
                        this.no_platforms_selected_label.set_visible(true);
                        this.no_platforms_selected_label.announce(
                            &this.no_platforms_selected_label.text(),
                            gtk::AccessibleAnnouncementPriority::High,
                        );
                    }
                    this.search_button.set_visible(true);
                    this.loading_spinner.set_visible(false);
                }
            }
        ));

        // create platform settings switches
        let settings = dbqueries::get_discovery_settings();
        for id in ALL_PLATFORM_IDS {
            let switch = adw::SwitchRow::new();
            let active = *settings.get(id).unwrap_or(&false);
            switch.set_active(active);
            switch.set_title(id);
            switch.set_selectable(false);
            switch.connect_active_notify(clone!(
                #[weak(rename_to = this)]
                self,
                move |s| {
                    if let Err(e) = dbqueries::set_discovery_setting(id, s.is_active()) {
                        error!("failed setting search preference: {e}");
                    } else if s.is_active() {
                        this.entry.remove_css_class("error");
                        this.no_platforms_selected_label.set_visible(false);
                    }
                }
            ));
            self.list.add(&switch);
        }

        self.entry.connect_activate(clone!(
            #[weak(rename_to = this)]
            self,
            #[strong]
            sender,
            move |entry| {
                let entry_text = entry.text().to_string();
                let url = Url::parse(&entry_text);
                let this = this.clone();
                this.search_button.set_visible(false);
                this.loading_spinner.set_visible(true);
                this.loading_spinner.announce(
                    &this
                        .loading_spinner
                        .tooltip_text()
                        .unwrap_or(gettext("Loading…").into()),
                    gtk::AccessibleAnnouncementPriority::High,
                );
                this.entry.remove_css_class("error");
                this.no_platforms_selected_label.set_visible(false);
                let loading_done = loading_done.clone();
                crate::RUNTIME.spawn(clone!(
                    #[strong]
                    sender,
                    async move {
                        if let Err(e) = match url {
                            Ok(url) => add_podcast_from_url(url.to_string(), &sender)
                                .await
                                .map_err(SearchError::from),
                            Err(_) => search_podcasts(entry_text, &sender).await,
                        } {
                            match e {
                                NoSearchPlatformsSelected => (),
                                _ => send!(sender, Action::ErrorNotification(format!("{e}"))),
                            };
                            send!(loading_done, Err(e));
                        } else {
                            send!(loading_done, Ok(()))
                        }
                    }
                ));
            }
        ));

        self.search_button.connect_clicked(clone!(
            #[weak(rename_to = this)]
            self,
            move |_| {
                this.entry.emit_activate();
            }
        ));
    }
}

#[glib::object_subclass]
impl ObjectSubclass for DiscoveryPagePriv {
    const NAME: &'static str = "PdDiscoveryPage";
    type Type = DiscoveryPage;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}
impl WidgetImpl for DiscoveryPagePriv {}
impl ObjectImpl for DiscoveryPagePriv {}
impl NavigationPageImpl for DiscoveryPagePriv {}
glib::wrapper! {
    pub struct DiscoveryPage(ObjectSubclass<DiscoveryPagePriv>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
impl DiscoveryPage {
    pub(crate) fn new(sender: &Sender<Action>) -> Self {
        let widget: Self = glib::Object::new();
        widget.imp().init(sender);

        widget
    }
}

async fn add_podcast_from_url(url_input: String, sender: &Sender<Action>) -> Result<()> {
    let mut url = url_input;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        url = format!("http://{}", url);
    };

    debug!("Url: {}", url);
    let url = if url.contains("itunes.com") || url.contains("apple.com") {
        info!("Detected itunes url.");
        let itunes_url = itunes_to_rss(&url).await?;
        info!("Resolved to {}", itunes_url);
        itunes_url
    } else if url.contains("soundcloud.com") && !url.contains("feeds.soundcloud.com") {
        info!("Detected soundcloud url.");
        let soundcloud_url = soundcloud_to_rss(&Url::parse(&url)?).await?;
        info!("Resolved to {}", soundcloud_url);
        soundcloud_url.to_string()
    } else {
        url.to_owned()
    };

    crate::utils::subscribe(sender, url).await;
    Ok(())
}

async fn search_podcasts(text: String, sender: &Sender<Action>) -> Result<(), SearchError> {
    let results = search(&text).await;
    send!(sender, Action::GoToFoundPodcasts(Arc::new(results?)));
    Ok(())
}

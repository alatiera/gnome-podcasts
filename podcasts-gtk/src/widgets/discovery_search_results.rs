// discovery_search_result.rs
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
use async_channel::Sender;
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::app::Action;
use podcasts_data::discovery::FoundPodcast;

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/discovery_search_results.ui")]
pub struct SearchResultsPriv {
    #[template_child]
    list: TemplateChild<gtk::ListBox>,
    #[template_child]
    no_results: TemplateChild<gtk::Label>,
}

impl SearchResultsPriv {
    pub(crate) fn init(&self, entries: &Vec<FoundPodcast>, sender: &Sender<Action>) {
        for e in entries {
            let entry_widget = Podcast::new(e, sender);
            self.list.append(&entry_widget);
        }
        if entries.is_empty() {
            self.no_results.set_visible(true);
            self.list.set_visible(false);
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SearchResultsPriv {
    const NAME: &'static str = "PdDiscoverySearchResults";
    type Type = SearchResults;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}
impl WidgetImpl for SearchResultsPriv {}
impl ObjectImpl for SearchResultsPriv {}
impl NavigationPageImpl for SearchResultsPriv {}
glib::wrapper! {
    pub struct SearchResults(ObjectSubclass<SearchResultsPriv>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SearchResults {
    pub(crate) fn new(entries: &Vec<FoundPodcast>, sender: &Sender<Action>) -> Self {
        let widget: Self = glib::Object::new();
        widget.imp().init(entries, sender);

        widget
    }
}

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/discovery_found_podcast.ui")]
pub struct PodcastPriv {
    #[template_child]
    subscribe: TemplateChild<gtk::Button>,
    #[template_child]
    cover: TemplateChild<gtk::Image>,
    #[template_child]
    description: TemplateChild<gtk::Label>,
    #[template_child]
    feed_url: TemplateChild<gtk::Label>,
    #[template_child]
    title: TemplateChild<gtk::Label>,
    #[template_child]
    author: TemplateChild<gtk::Label>,
    #[template_child]
    episode_count: TemplateChild<gtk::Box>,
    #[template_child]
    episode_count_label: TemplateChild<gtk::Label>,
    #[template_child]
    last_publication: TemplateChild<gtk::Label>,
}

impl PodcastPriv {
    fn init(&self, p: &FoundPodcast, sender: &Sender<Action>) {
        self.title.set_text(&p.title);
        self.feed_url.set_text(&p.feed);
        self.author.set_text(&p.author);

        let description = p.description.trim();
        if !description.is_empty() {
            self.description.set_text(description);
            self.description.set_tooltip_text(Some(description));
            self.description.set_visible(true);
        }
        if let Some(ep_count) = p.episode_count {
            self.episode_count_label.set_text(&format!("{}", ep_count));
            self.episode_count_label.set_visible(true);
        }
        if let Some(last_publication) = p.last_publication {
            let date = last_publication.format("%e %b %Y").to_string();
            self.last_publication.set_text(&date);
            self.last_publication.set_visible(true);
        }

        let feed = p.feed.clone();
        let sender = sender.clone();
        self.subscribe.connect_clicked(move |_| {
            send_blocking!(sender, Action::Subscribe(feed.clone()));
        });

        let art = p.art.clone();
        let (sender, receiver) = async_channel::bounded(1);
        crate::RUNTIME.spawn(async move {
            if let Err(e) = async {
                let response = reqwest::get(&art).await?;
                let bytes = response.bytes().await?;
                let texture = {
                    let strm = gtk::gio::MemoryInputStream::from_bytes(&glib::Bytes::from(&bytes));
                    let pixbuf =
                        gtk::gdk_pixbuf::Pixbuf::from_stream(&strm, gtk::gio::Cancellable::NONE)?;
                    gtk::gdk::Texture::for_pixbuf(&pixbuf)
                };
                sender
                    .send(texture)
                    .await
                    .expect("failed to send img to main thread");
                Ok::<(), anyhow::Error>(())
            }
            .await
            {
                error!("failed to load image for search result: {art} {e}");
            }
        });

        crate::MAINCONTEXT.spawn_local(clone!(@weak self as this => async move {
            if let Ok(texture) = receiver.recv().await {
                this.cover.set_from_paintable(Some(&texture));
            }
        }));
    }
}

#[glib::object_subclass]
impl ObjectSubclass for PodcastPriv {
    const NAME: &'static str = "PdDiscoveryFoundPodcast";
    type Type = Podcast;
    type ParentType = adw::PreferencesRow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}
impl ObjectImpl for PodcastPriv {}
impl WidgetImpl for PodcastPriv {}
impl ListBoxRowImpl for PodcastPriv {}
impl PreferencesRowImpl for PodcastPriv {}

glib::wrapper! {
    pub struct Podcast(ObjectSubclass<PodcastPriv>)
        @extends adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Actionable, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Podcast {
    pub(crate) fn new(p: &FoundPodcast, sender: &Sender<Action>) -> Self {
        let widget: Self = glib::Object::new();
        widget.imp().init(p, sender);
        widget
    }
}

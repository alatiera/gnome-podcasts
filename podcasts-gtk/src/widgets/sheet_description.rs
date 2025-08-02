// chapters_page.rs
//
// Copyright 2025 nee <nee-git@patchouli.garden>
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
use async_channel::Sender;
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::Cell;

use crate::app::Action;
use crate::episode_description_parser;
use podcasts_data::{Episode, EpisodeId, ShowCoverModel};

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/sheet_description.ui")]
pub struct SheetDescriptionPriv {
    #[template_child]
    episode: TemplateChild<gtk::Label>,
    #[template_child]
    show: TemplateChild<gtk::Label>,
    #[template_child]
    description: TemplateChild<gtk::Label>,

    id: Cell<Option<EpisodeId>>,
}

#[glib::object_subclass]
impl ObjectSubclass for SheetDescriptionPriv {
    const NAME: &'static str = "PdSheetDescription";
    type Type = SheetDescription;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for SheetDescriptionPriv {}
impl ObjectImpl for SheetDescriptionPriv {}
impl BinImpl for SheetDescriptionPriv {}

glib::wrapper! {
    pub struct SheetDescription(ObjectSubclass<SheetDescriptionPriv>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SheetDescription {
    pub(crate) fn init(&self, sender: &Sender<Action>) {
        self.imp().description.connect_activate_link(clone!(
            #[weak(rename_to=this)]
            self,
            #[strong]
            sender,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, url| {
                if let Some(seconds_str) = url.strip_prefix("jump:") {
                    if let Ok(seconds) = seconds_str.parse() {
                        if let Some(id) = this.imp().id.get() {
                            send_blocking!(sender, Action::InitEpisodeAt(id, seconds));
                        }
                    } else {
                        error!("failed to parse jump link: {url}");
                    }
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));
    }

    pub fn initialize_episode(&self, ep: &Episode, show: &ShowCoverModel) {
        self.imp().id.set(Some(ep.id()));
        self.imp().episode.set_label(ep.title());
        self.imp().show.set_label(show.title());
        self.set_description(ep);
    }

    fn set_description(&self, ep: &Episode) {
        if let Some(t) = ep.description() {
            let imp = self.imp();
            let default_text = imp.description.text();

            let markup = episode_description_parser::html2pango_markup(t);
            imp.description.set_markup(&markup);
            // recover from invalid markup
            if imp.description.text() == default_text {
                let plain = html2text::config::plain()
                    .string_from_read(t.as_bytes(), t.len())
                    .unwrap_or_else(|_| t.to_string());
                imp.description.set_text(&plain);
            }
        };
    }
}

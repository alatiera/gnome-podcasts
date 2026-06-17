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

use adw::prelude::*;
use adw::subclass::prelude::*;
use async_channel::Sender;
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::app::Action;
use crate::chapter_parser::Chapter;
use crate::player::Player;
use crate::widgets::PlayerRate;
use crate::widgets::{Chapters, SheetDescription, SheetPlayer};
use podcasts_data::EpisodeId;

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/sheet_base.ui")]
pub(crate) struct SheetBasePriv {
    #[template_child]
    pub(crate) stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    pub(crate) toggle_group: TemplateChild<adw::ToggleGroup>,
    #[template_child]
    pub(crate) player: TemplateChild<SheetPlayer>,
    #[template_child]
    pub(crate) description: TemplateChild<SheetDescription>,
    #[template_child]
    pub(crate) chapters: TemplateChild<Chapters>,
    #[template_child]
    pub(crate) chapters_toggle: TemplateChild<adw::Toggle>,
    #[template_child]
    pub(crate) rate: TemplateChild<PlayerRate>,
}

#[glib::object_subclass]
impl ObjectSubclass for SheetBasePriv {
    const NAME: &'static str = "PdSheetBase";
    type Type = SheetBase;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        SheetPlayer::ensure_type();
        SheetDescription::ensure_type();
        Chapters::ensure_type();
        PlayerRate::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for SheetBasePriv {}
impl ObjectImpl for SheetBasePriv {}
impl BinImpl for SheetBasePriv {}

glib::wrapper! {
    pub(crate) struct SheetBase(ObjectSubclass<SheetBasePriv>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SheetBase {
    pub(crate) fn init(&self, player: &Player, sender: &Sender<Action>) {
        let imp = self.imp();
        imp.toggle_group.connect_active_name_notify(clone!(
            #[weak(rename_to = this)]
            self,
            move |group| if let Some(name) = group.active_name() {
                this.imp().stack.set_visible_child_name(&name)
            },
        ));
        imp.description.init(sender);
        imp.chapters.init(player, sender);
        imp.rate.init(player);
        imp.player.init(player);

        player.connect_local(
            "episode-changed",
            false,
            clone!(
                #[weak(rename_to = this)]
                self,
                #[weak]
                player,
                #[upgrade_or_default]
                move |_| {
                    if let Some(episode) = player.episode().as_ref()
                        && let Some(show) = player.show().as_ref()
                    {
                        this.imp().description.initialize_episode(episode, show);
                    }
                    this.imp().toggle_group.set_active_name(Some("player"));
                    None
                }
            ),
        );

        player.connect_local(
            "chapters-changed",
            false,
            clone!(
                #[weak(rename_to = this)]
                self,
                #[weak]
                player,
                #[upgrade_or_default]
                move |_| {
                    if let Some(id) = player.episode_id() {
                        this.chapters_changed(id, player.chapters());
                    } else {
                        this.imp().chapters_toggle.set_enabled(false);
                    }
                    None
                }
            ),
        );
    }

    pub(crate) fn on_open_changed(&self, is_open: bool) {
        if !is_open {
            self.imp().toggle_group.set_active_name(Some("player"));
        }
    }

    pub(crate) fn chapters_changed(&self, id: EpisodeId, chapters: Vec<Chapter>) {
        let imp = self.imp();
        let has_chapters = !chapters.is_empty();
        imp.chapters_toggle.set_enabled(has_chapters);
        if has_chapters {
            imp.chapters.set_chapters(id, chapters);
        }
    }
}

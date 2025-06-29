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
use gtk::gio;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

use crate::app::Action;
use crate::chapter_parser::Chapter;
use crate::widgets::player::{PlayerRate, PlayerTimes, PlayerWidget};
use crate::widgets::{Chapters, SheetDescription, SheetPlayer};
use podcasts_data::{Episode, EpisodeId, ShowCoverModel};

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/sheet_base.ui")]
pub(crate) struct SheetBasePriv {
    #[template_child]
    pub(crate) rate_container: TemplateChild<adw::Bin>,
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

    rate: RefCell<Option<PlayerRate>>,
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
    pub(crate) fn new(rate: PlayerRate) -> Self {
        let this: Self = glib::Object::new();
        let imp = this.imp();
        imp.rate_container.set_child(Some(&rate.btn));
        imp.rate.replace(Some(rate));

        imp.toggle_group.connect_active_name_notify(clone!(
            #[weak]
            this,
            move |group| if let Some(name) = group.active_name() {
                this.imp().stack.set_visible_child_name(&name)
            },
        ));

        this
    }

    pub(crate) fn init(&self, sender: &Sender<Action>, progress: &gtk::Scale) {
        self.imp().description.init(sender);
        self.imp().chapters.init(sender, progress);
    }

    pub(crate) fn initialize_episode(&self, episode: &Episode, show: &ShowCoverModel) {
        let imp = self.imp();

        imp.chapters_toggle.set_enabled(false);
        imp.player.initialize_episode(episode, show);
        imp.description.initialize_episode(episode, show);
        imp.toggle_group.set_active_name(Some("player"));
    }

    pub(crate) fn on_rate_changed(&self, rate: f64) {
        self.rate().btn.set_label(&format!("{:.2}×", rate));
    }

    pub(crate) fn on_open_changed(&self, is_open: bool) {
        if !is_open {
            self.imp().toggle_group.set_active_name(Some("player"));
        }
    }

    pub(crate) fn chapters_available(&self, id: EpisodeId, chapters: Vec<Chapter>) {
        if chapters.is_empty() {
            return;
        }
        let imp = self.imp();
        imp.chapters_toggle.set_enabled(true);
        imp.chapters.set_chapters(id, chapters);
    }

    pub(crate) fn on_play(&self) {
        self.imp().player.on_play();
    }

    pub(crate) fn on_pause(&self) {
        self.imp().player.on_pause();
    }

    pub(crate) fn on_stop(&self) {
        self.imp().player.on_stop();
    }

    pub(crate) fn connect(&self, timer: &PlayerTimes) {
        self.imp().player.connect(timer);
    }

    pub(crate) fn connect_rate_actions(&self, group: &gio::SimpleActionGroup) {
        self.insert_action_group("rate", Some(group));
    }

    pub(crate) fn connect_control_buttons(&self, player: &Rc<RefCell<PlayerWidget>>) {
        self.imp().player.connect_control_buttons(player);
    }

    pub(crate) fn rate(&self) -> PlayerRate {
        self.imp().rate.borrow().as_ref().unwrap().clone()
    }
}

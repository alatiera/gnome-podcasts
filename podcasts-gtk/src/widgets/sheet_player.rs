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
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

use crate::download_covers::load_widget_texture;
use crate::widgets::player::PlayerExt;
use crate::widgets::player::{PlayerTimes, PlayerWidget};
use podcasts_data::Episode;
use podcasts_data::ShowCoverModel;

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/sheet_player.ui")]
pub(crate) struct SheetPlayerPriv {
    #[template_child]
    cover: TemplateChild<gtk::Image>,
    #[template_child]
    play_pause: TemplateChild<gtk::Stack>,
    #[template_child]
    play: TemplateChild<gtk::Button>,
    #[template_child]
    pause: TemplateChild<gtk::Button>,
    #[template_child]
    duration: TemplateChild<gtk::Label>,
    #[template_child]
    progressed: TemplateChild<gtk::Label>,
    #[template_child]
    slider: TemplateChild<gtk::Scale>,
    #[template_child]
    forward: TemplateChild<gtk::Button>,
    #[template_child]
    rewind: TemplateChild<gtk::Button>,
    #[template_child]
    show: TemplateChild<gtk::Label>,
    #[template_child]
    episode: TemplateChild<gtk::Label>,
}

#[glib::object_subclass]
impl ObjectSubclass for SheetPlayerPriv {
    const NAME: &'static str = "PdSheetPlayer";
    type Type = SheetPlayer;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for SheetPlayerPriv {}
impl ObjectImpl for SheetPlayerPriv {}
impl BinImpl for SheetPlayerPriv {}

glib::wrapper! {
    pub(crate) struct SheetPlayer(ObjectSubclass<SheetPlayerPriv>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SheetPlayer {
    pub(crate) fn connect(&self, timer: &PlayerTimes) {
        let imp = self.imp();
        timer
            .duration
            .bind_property("label", &imp.duration.get(), "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        timer
            .progressed
            .bind_property("label", &imp.progressed.get(), "label")
            .flags(glib::BindingFlags::SYNC_CREATE)
            .build();
        imp.slider.set_adjustment(&timer.slider.adjustment());
    }

    pub(crate) fn initialize_episode(&self, episode: &Episode, show: &ShowCoverModel) {
        let imp = self.imp();
        imp.episode.set_text(episode.title());
        imp.show.set_text(show.title());
        load_widget_texture(&imp.cover.get(), show.id(), crate::Thumb256, true);
    }

    pub(crate) fn on_play(&self) {
        let imp = self.imp();
        imp.play_pause.set_visible_child(&imp.pause.get());
    }

    pub(crate) fn on_pause(&self) {
        let imp = self.imp();
        imp.play_pause.set_visible_child(&imp.play.get());
    }

    pub(crate) fn on_stop(&self) {
        let imp = self.imp();
        let is_focus = imp.pause.is_focus();
        imp.play_pause.set_visible_child(&imp.play.get());
        if is_focus {
            imp.play.grab_focus();
        }
    }

    pub(crate) fn connect_control_buttons(&self, player: &Rc<RefCell<PlayerWidget>>) {
        let imp = self.imp();
        // Connect buttons to gst Player
        imp.play.connect_clicked(clone!(
            #[weak]
            player,
            #[weak(rename_to=this)]
            self,
            move |_| {
                player.borrow().play();
                this.imp().pause.grab_focus(); // keep focus for accessibility
            }
        ));

        imp.pause.connect_clicked(clone!(
            #[weak]
            player,
            #[weak(rename_to=this)]
            self,
            move |_| {
                player.borrow_mut().pause();
                this.imp().play.grab_focus(); // keep focus for accessibility
            }
        ));

        imp.rewind.connect_clicked(clone!(
            #[weak]
            player,
            move |_| {
                player.borrow().rewind();
            }
        ));

        imp.forward.connect_clicked(clone!(
            #[weak]
            player,
            move |_| {
                player.borrow().fast_forward();
            }
        ));
    }
}

// player_big.rs
//
// Copyright 2018 Jordan Petridis <jpetridis@gnome.org>
// Copyright 2021-2026 nee <nee-git@patchouli.garden>
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
use glib::SignalHandlerId;
use glib::clone;
use gst::ClockTime;
use gtk::CompositeTemplate;
use gtk::TemplateChild;
use gtk::glib;
use mpris_server::PlaybackStatus;
use std::cell::RefCell;

use crate::download_covers::load_widget_texture;
use crate::player::{Duration, Player, PlayerUi, Position};
use crate::utils::format_duration;
use crate::widgets::PlayerRate;
use podcasts_data::{Episode, ShowCoverModel};

#[derive(Debug, Default, CompositeTemplate, glib::Properties)]
#[template(resource = "/org/gnome/Podcasts/gtk/player_big.ui")]
#[properties(wrapper_type = PlayerBig)]
pub struct PlayerBigPriv {
    #[template_child]
    play: TemplateChild<gtk::Button>,
    #[template_child]
    pause: TemplateChild<gtk::Button>,
    #[template_child]
    play_pause: TemplateChild<gtk::Stack>,
    #[template_child]
    forward: TemplateChild<gtk::Button>,
    #[template_child]
    rewind: TemplateChild<gtk::Button>,
    #[template_child]
    chapters_button: TemplateChild<gtk::Button>,

    #[template_child]
    show: TemplateChild<gtk::Label>,
    #[template_child]
    episode: TemplateChild<gtk::Label>,
    #[template_child]
    cover: TemplateChild<gtk::Image>,
    #[template_child]
    cover_button: TemplateChild<gtk::Button>,

    #[template_child]
    progressed: TemplateChild<gtk::Label>,
    #[template_child]
    duration: TemplateChild<gtk::Label>,
    #[template_child]
    slider: TemplateChild<gtk::Scale>,
    #[template_child]
    rate: TemplateChild<PlayerRate>,

    // for blocking the signal during duration/position updates
    // as the signal is used to jump when the slider is dragged by a user
    slider_update: RefCell<Option<SignalHandlerId>>,
}

#[glib::object_subclass]
impl ObjectSubclass for PlayerBigPriv {
    const NAME: &'static str = "PdPlayerBig";
    type Type = super::PlayerBig;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        PlayerRate::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for PlayerBigPriv {}
impl WidgetImpl for PlayerBigPriv {}
impl BoxImpl for PlayerBigPriv {}
glib::wrapper! {
    pub struct PlayerBig(ObjectSubclass<PlayerBigPriv>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible, gtk::Orientable;
}

impl PlayerBig {
    pub fn init(&self, player: &Player) {
        let imp = self.imp();
        imp.slider.set_range(0.0, 1.0);
        let slider_update = Self::connect_update_slider(&imp.slider, player);
        imp.slider_update.replace(Some(slider_update));

        imp.rate.init(player);
        player.bind_ui(self);
    }

    fn connect_update_slider(slider: &gtk::Scale, player: &Player) -> SignalHandlerId {
        slider.connect_value_changed(clone!(
            #[weak]
            player,
            move |slider| {
                let value = slider.value() as u64;
                player.jump_to(Position(ClockTime::from_seconds(value)));
            }
        ))
    }
}

impl PlayerUi for PlayerBig {
    fn show_cover_changed(&self, show: &ShowCoverModel) {
        load_widget_texture(&self.imp().cover.get(), show.id(), crate::Thumb64, false);
    }

    fn show_cover_reset(&self) {
        self.imp()
            .cover
            .set_icon_name(Some("image-missing-symbolic"));
    }

    fn show_changed(&self, show: &ShowCoverModel) {
        self.imp().show.set_text(show.title());
        self.imp().show.set_tooltip_text(Some(show.title()));
    }

    fn episode_changed(&self, ep: &Episode) {
        let imp = self.imp();
        imp.episode.set_text(ep.title());
        imp.episode.set_tooltip_text(Some(ep.title()));

        imp.cover_button
            .set_action_target_value(Some(&ep.id().0.into()));
        imp.cover_button.set_action_name(Some("app.go-to-episode"));
    }

    fn status_changed(&self, status: PlaybackStatus) {
        let stack = &self.imp().play_pause;
        let had_focus = stack
            .visible_child()
            .map(|w| w.is_focus())
            .unwrap_or_default();
        let new_button = match status {
            PlaybackStatus::Paused => self.imp().play.get(),
            PlaybackStatus::Stopped => self.imp().play.get(),
            _ => self.imp().pause.get(),
        };
        stack.set_visible_child(&new_button);
        // restore focus for accessibility
        if had_focus {
            new_button.grab_focus();
        }
    }

    fn position_changed(&self, position: Position) {
        let seconds = position.seconds();
        let imp = self.imp();
        imp.slider
            .block_signal(imp.slider_update.borrow().as_ref().unwrap());
        imp.slider.set_value(seconds as f64);
        imp.slider
            .unblock_signal(imp.slider_update.borrow().as_ref().unwrap());

        imp.progressed.set_text(&format_duration(seconds as u32));
    }
    fn duration_changed(&self, duration: Duration) {
        let seconds = duration.seconds();
        let imp = self.imp();
        imp.slider
            .block_signal(imp.slider_update.borrow().as_ref().unwrap());
        imp.slider.set_range(0.0, seconds as f64);
        imp.slider
            .unblock_signal(imp.slider_update.borrow().as_ref().unwrap());

        imp.duration.set_text(&format_duration(seconds as u32));
    }

    fn chapters_changed(&self, has_chapters: bool) {
        self.imp().chapters_button.set_visible(has_chapters);
    }
}

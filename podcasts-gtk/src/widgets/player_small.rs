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
use gtk::CompositeTemplate;
use gtk::TemplateChild;
use gtk::glib;
use mpris_server::PlaybackStatus;
use std::cell::Cell;

use crate::download_covers::load_widget_texture;
use crate::player::{Duration, Player, PlayerUi, Position};
use podcasts_data::{Episode, ShowCoverModel};

#[derive(Debug, Default, CompositeTemplate, glib::Properties)]
#[template(resource = "/org/gnome/Podcasts/gtk/player_small.ui")]
#[properties(wrapper_type = PlayerSmall)]
pub struct PlayerSmallPriv {
    #[template_child]
    show: TemplateChild<gtk::Label>,
    #[template_child]
    episode: TemplateChild<gtk::Label>,
    #[template_child]
    cover: TemplateChild<gtk::Image>,
    #[template_child]
    cover_button: TemplateChild<gtk::Button>,
    #[template_child]
    play: TemplateChild<gtk::Button>,
    #[template_child]
    pause: TemplateChild<gtk::Button>,
    #[template_child]
    play_pause: TemplateChild<gtk::Stack>,
    #[template_child]
    progress_bar: TemplateChild<gtk::ProgressBar>,

    duration: Cell<f64>,
}

#[glib::object_subclass]
impl ObjectSubclass for PlayerSmallPriv {
    const NAME: &'static str = "PdPlayerSmall";
    type Type = super::PlayerSmall;
    type ParentType = gtk::Box;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for PlayerSmallPriv {}
impl WidgetImpl for PlayerSmallPriv {}
impl BoxImpl for PlayerSmallPriv {}
glib::wrapper! {
    pub struct PlayerSmall(ObjectSubclass<PlayerSmallPriv>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible, gtk::Orientable;
}

impl PlayerSmall {
    pub fn init(&self, player: &Player) {
        player.bind_ui(self);
    }
}

impl PlayerUi for PlayerSmall {
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
        let fraction = seconds as f64 / self.imp().duration.get();
        self.imp()
            .progress_bar
            .set_fraction(if fraction.is_nan() { 0.0 } else { fraction });
    }
    fn duration_changed(&self, duration: Duration) {
        self.imp().duration.set(duration.seconds() as f64);
    }
}

// player.rs
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
use glib::clone;
use gtk::CompositeTemplate;
use gtk::TemplateChild;
use gtk::glib;

use crate::player::Player;
use crate::widgets::{PlayerBig, PlayerSmall};

#[derive(Debug, Default, CompositeTemplate, glib::Properties)]
#[template(resource = "/org/gnome/Podcasts/gtk/player_wrapper.ui")]
#[properties(wrapper_type = PlayerWrapper)]
pub struct PlayerWrapperPriv {
    #[template_child]
    stack: TemplateChild<gtk::Stack>,
    #[template_child]
    big: TemplateChild<PlayerBig>,
    #[template_child]
    small: TemplateChild<PlayerSmall>,
}

#[glib::object_subclass]
impl ObjectSubclass for PlayerWrapperPriv {
    const NAME: &'static str = "PdPlayerWrapper";
    type Type = super::PlayerWrapper;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        PlayerBig::ensure_type();
        PlayerSmall::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for PlayerWrapperPriv {}
impl WidgetImpl for PlayerWrapperPriv {}
impl BinImpl for PlayerWrapperPriv {}
glib::wrapper! {
    pub struct PlayerWrapper(ObjectSubclass<PlayerWrapperPriv>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for PlayerWrapper {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl PlayerWrapper {
    fn reveal(&self) {
        self.set_visible(true);
    }

    pub(crate) fn set_small(&self, small: bool) {
        if small {
            self.imp().stack.set_visible_child(&self.imp().small.get());
        } else {
            self.imp().stack.set_visible_child(&self.imp().big.get());
        }
    }

    pub(crate) fn init(&self, player: &Player) {
        player.connect_local(
            "episode-changed",
            false,
            clone!(
                #[weak(rename_to = this)]
                self,
                #[upgrade_or_default]
                move |_| {
                    this.reveal();
                    None
                }
            ),
        );

        self.imp().small.init(player);
        self.imp().big.init(player);
    }
}

// player_rate.rs
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
use formatx::formatx;
use gettextrs::gettext;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::TemplateChild;
use gtk::glib;

use crate::player::Player;

#[derive(Debug, Default, CompositeTemplate, glib::Properties)]
#[template(resource = "/org/gnome/Podcasts/gtk/player_rate.ui")]
#[properties(wrapper_type = PlayerRate)]
pub struct PlayerRatePriv {
    #[template_child]
    button: TemplateChild<gtk::MenuButton>,
}

#[glib::object_subclass]
impl ObjectSubclass for PlayerRatePriv {
    const NAME: &'static str = "PdPlayerRate";
    type Type = super::PlayerRate;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        PlayerRate::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for PlayerRatePriv {}
impl WidgetImpl for PlayerRatePriv {}
impl BinImpl for PlayerRatePriv {}
glib::wrapper! {
    pub struct PlayerRate(ObjectSubclass<PlayerRatePriv>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PlayerRate {
    pub fn init(&self, player: &Player) {
        player.connect_local(
            "rate-changed",
            false,
            clone!(
                #[weak(rename_to = this)]
                self,
                #[upgrade_or_default]
                move |value| {
                    if let Ok(rate) = value[1].get::<f64>() {
                        // Translators: This will show as something like: "1.25×"
                        // inside a menu-button that shows/sets playback speed.
                        let label = formatx!(gettext("{:.2}×"), rate)
                            .expect("Could not format translatable string");
                        this.imp().button.set_label(&label);
                    } else {
                        error!("unknown playback rate value in signal to rate widget");
                    }
                    None
                }
            ),
        );
    }
}

// base_view.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
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

use gtk::PolicyType;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use adw::prelude::*;
use adw::subclass::prelude::*;

use std::sync::LazyLock;

#[derive(Debug, Default)]
pub struct BaseViewPriv {
    pub scrolled_window: gtk::ScrolledWindow,
}

#[glib::object_subclass]
impl ObjectSubclass for BaseViewPriv {
    const NAME: &'static str = "PdBaseView";
    type Type = super::BaseView;
    type ParentType = adw::Bin;
}

impl ObjectImpl for BaseViewPriv {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();

        self.scrolled_window
            .set_policy(PolicyType::Never, PolicyType::Automatic);
        obj.set_size_request(360, -1);
        obj.set_child(Some(&self.scrolled_window));
    }

    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecObject::builder::<gtk::Widget>("child")
                    .readwrite()
                    .build(),
            ]
        });
        PROPERTIES.as_ref()
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "child" => self.scrolled_window.child().to_value(),
            _ => unimplemented!(),
        }
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "child" => self
                .scrolled_window
                .set_child(value.get::<gtk::Widget>().ok().as_ref()),
            _ => unimplemented!(),
        };
    }
}

impl WidgetImpl for BaseViewPriv {}
impl BinImpl for BaseViewPriv {}

glib::wrapper! {
    pub struct BaseView(ObjectSubclass<BaseViewPriv>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl Default for BaseView {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl BaseView {
    pub(crate) fn set_content<T: IsA<gtk::Widget>>(&self, widget: &T) {
        self.imp().scrolled_window.set_child(Some(widget));
    }
}

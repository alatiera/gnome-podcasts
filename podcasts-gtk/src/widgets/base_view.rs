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

use crate::utils::smooth_scroll_to;

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{Adjustment, PolicyType};

use adw::prelude::*;
use adw::subclass::prelude::*;

use once_cell::sync::Lazy;

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
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        self.scrolled_window
            .set_policy(PolicyType::Never, PolicyType::Automatic);
        obj.set_size_request(360, -1);
        obj.set_child(Some(&self.scrolled_window));
    }

    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![glib::ParamSpec::new_object(
                "child",
                "child",
                "child",
                gtk::Widget::static_type(),
                glib::ParamFlags::READWRITE,
            )]
        });
        PROPERTIES.as_ref()
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "child" => self.scrolled_window.child().to_value(),
            _ => unimplemented!(),
        }
    }

    fn set_property(
        &self,
        _obj: &Self::Type,
        _id: usize,
        value: &glib::Value,
        pspec: &glib::ParamSpec,
    ) {
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
        @extends gtk::Widget, adw::Bin;
}

impl Default for BaseView {
    fn default() -> Self {
        glib::Object::new(&[]).unwrap()
    }
}

impl BaseView {
    pub(crate) fn set_content<T: IsA<gtk::Widget>>(&self, widget: &T) {
        let self_ = BaseViewPriv::from_instance(&self);

        self_.scrolled_window.set_child(Some(widget));
    }

    pub(crate) fn set_adjustments(
        &self,
        hadjustment: Option<&Adjustment>,
        vadjustment: Option<&Adjustment>,
    ) {
        let self_ = BaseViewPriv::from_instance(&self);

        if let Some(h) = hadjustment {
            smooth_scroll_to(&self_.scrolled_window, h);
        }

        if let Some(v) = vadjustment {
            smooth_scroll_to(&self_.scrolled_window, v);
        }
    }

    pub(crate) fn vadjustment(&self) -> Adjustment {
        let self_ = BaseViewPriv::from_instance(&self);

        self_.scrolled_window.vadjustment().unwrap()
    }
}

// home_view.rs
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

use anyhow::Result;
use chrono::prelude::*;

use adw::subclass::prelude::*;
use glib::clone;
use glib::subclass::InitializingObject;
use glib::Sender;
use gtk::{glib, prelude::*, Adjustment, CompositeTemplate};

use podcasts_data::dbqueries;
use podcasts_data::EpisodeWidgetModel;

use crate::app::Action;
use crate::utils::{self, lazy_load_full};
use crate::widgets::{BaseView, EpisodeWidget};

#[derive(Debug, Clone)]
enum ListSplit {
    Today,
    Yday,
    Week,
    Month,
    Rest,
}

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/home_view.ui")]
pub struct HomeViewPriv {
    #[template_child]
    view: TemplateChild<BaseView>,
    #[template_child]
    today_box: TemplateChild<gtk::Box>,
    #[template_child]
    yday_box: TemplateChild<gtk::Box>,
    #[template_child]
    week_box: TemplateChild<gtk::Box>,
    #[template_child]
    month_box: TemplateChild<gtk::Box>,
    #[template_child]
    rest_box: TemplateChild<gtk::Box>,
    #[template_child]
    today_list: TemplateChild<gtk::ListBox>,
    #[template_child]
    yday_list: TemplateChild<gtk::ListBox>,
    #[template_child]
    week_list: TemplateChild<gtk::ListBox>,
    #[template_child]
    month_list: TemplateChild<gtk::ListBox>,
    #[template_child]
    rest_list: TemplateChild<gtk::ListBox>,
}

#[glib::object_subclass]
impl ObjectSubclass for HomeViewPriv {
    const NAME: &'static str = "PdHomeView";
    type Type = HomeView;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        BaseView::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for HomeViewPriv {}
impl ObjectImpl for HomeViewPriv {}
impl BinImpl for HomeViewPriv {}

glib::wrapper! {
    pub struct HomeView(ObjectSubclass<HomeViewPriv>)
        @extends BaseView, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl HomeView {
    pub(crate) fn new(sender: Sender<Action>, vadj: Option<Adjustment>) -> Result<Self> {
        use self::ListSplit::*;

        let home: Self = glib::Object::new();

        let ignore = utils::get_ignored_shows()?;
        let episodes = dbqueries::get_episodes_widgets_filter_limit(&ignore, 100)?;
        let now_utc = Utc::now();

        let func = clone!(@weak home => move |ep: EpisodeWidgetModel| {
            let epoch = ep.epoch();
            let widget = HomeEpisode::new(ep, &sender);

            match split(&now_utc, i64::from(epoch)) {
                Today => add_to_box(&widget, &home.imp().today_list, &home.imp().today_box),
                Yday => add_to_box(&widget, &home.imp().yday_list, &home.imp().yday_box),
                Week => add_to_box(&widget, &home.imp().week_list, &home.imp().week_box),
                Month => add_to_box(&widget, &home.imp().month_list, &home.imp().month_box),
                Rest => add_to_box(&widget, &home.imp().rest_list, &home.imp().rest_box),
            }
        });

        let callback = clone!(@weak home => move || {
            if let Some(ref v) = vadj {
                home.imp().view.set_adjustments(None, Some(v))
            };
        });

        lazy_load_full(episodes, func, callback);
        Ok(home)
    }

    pub(crate) fn view(&self) -> &BaseView {
        &self.imp().view
    }
}

fn add_to_box(widget: &HomeEpisode, listbox: &gtk::ListBox, box_: &gtk::Box) {
    listbox.append(&widget.row);
    box_.set_visible(true);
}

fn split(now: &DateTime<Utc>, epoch: i64) -> ListSplit {
    let ep = Utc.timestamp_opt(epoch, 0).unwrap();
    let days_now = now.num_days_from_ce();
    let days_ep = ep.num_days_from_ce();
    let weekday = now.weekday().num_days_from_monday() as i32;

    if days_ep == days_now {
        ListSplit::Today
    } else if days_ep == days_now - 1 {
        ListSplit::Yday
    } else if days_ep >= days_now - weekday {
        ListSplit::Week
    } else if now.month() == ep.month() && now.year() == ep.year() {
        ListSplit::Month
    } else {
        ListSplit::Rest
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HomeEpisode {
    row: gtk::ListBoxRow,
    image: gtk::Image,
}

impl Default for HomeEpisode {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/home_episode.ui");
        let container: gtk::Box = builder.object("container").unwrap();
        let image: gtk::Image = builder.object("cover").unwrap();
        let ep = EpisodeWidget::default();
        container.append(&ep);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&container));
        row.set_visible(true);

        HomeEpisode { row, image }
    }
}

impl HomeEpisode {
    fn new(episode: EpisodeWidgetModel, sender: &Sender<Action>) -> HomeEpisode {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/home_episode.ui");
        let container: gtk::Box = builder.object("container").unwrap();
        let image: gtk::Image = builder.object("cover").unwrap();
        let pid = episode.show_id();
        let id = episode.rowid();
        let ep = EpisodeWidget::new(sender, &episode);
        container.append(&ep);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&container));
        row.set_action_name(Some("app.go-to-episode"));
        row.set_action_target_value(Some(&id.to_variant()));
        row.set_visible(true);

        let view = HomeEpisode { row, image };

        view.init(pid);
        view
    }

    fn init(&self, show_id: i32) {
        self.set_cover(show_id);
    }

    fn set_cover(&self, show_id: i32) {
        utils::set_image_from_path(&self.image, show_id, 64)
            .map_err(|err| error!("Failed to set a cover: {}", err))
            .ok();
    }
}

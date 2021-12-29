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

use gtk::{prelude::*, Adjustment};

use glib::clone;

use adw::Clamp;
use glib::Sender;
use podcasts_data::dbqueries;
use podcasts_data::EpisodeWidgetModel;

use crate::app::Action;
use crate::utils::{self, lazy_load_full};
use crate::widgets::{BaseView, EpisodeWidget};

use std::rc::Rc;

#[derive(Debug, Clone)]
enum ListSplit {
    Today,
    Yday,
    Week,
    Month,
    Rest,
}

#[derive(Debug, Clone)]
pub(crate) struct HomeView {
    pub(crate) view: BaseView,
    frame_parent: gtk::Box,
    today_box: gtk::Box,
    yday_box: gtk::Box,
    week_box: gtk::Box,
    month_box: gtk::Box,
    rest_box: gtk::Box,
    today_list: gtk::ListBox,
    yday_list: gtk::ListBox,
    week_list: gtk::ListBox,
    month_list: gtk::ListBox,
    rest_list: gtk::ListBox,
}

impl Default for HomeView {
    fn default() -> Self {
        let view = BaseView::default();
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/home_view.ui");
        let frame_parent: gtk::Box = builder.object("frame_parent").unwrap();
        let today_box: gtk::Box = builder.object("today_box").unwrap();
        let yday_box: gtk::Box = builder.object("yday_box").unwrap();
        let week_box: gtk::Box = builder.object("week_box").unwrap();
        let month_box: gtk::Box = builder.object("month_box").unwrap();
        let rest_box: gtk::Box = builder.object("rest_box").unwrap();
        let today_list: gtk::ListBox = builder.object("today_list").unwrap();
        let yday_list: gtk::ListBox = builder.object("yday_list").unwrap();
        let week_list: gtk::ListBox = builder.object("week_list").unwrap();
        let month_list: gtk::ListBox = builder.object("month_list").unwrap();
        let rest_list: gtk::ListBox = builder.object("rest_list").unwrap();

        let clamp = Clamp::new();
        clamp.show();
        clamp.set_maximum_size(700);

        clamp.set_child(Some(&frame_parent));
        view.set_content(&clamp);

        HomeView {
            view,
            frame_parent,
            today_box,
            yday_box,
            week_box,
            month_box,
            rest_box,
            today_list,
            yday_list,
            week_list,
            month_list,
            rest_list,
        }
    }
}

// TODO: REFACTOR ME
impl HomeView {
    pub(crate) fn new(sender: Sender<Action>, vadj: Option<Adjustment>) -> Result<Rc<HomeView>> {
        use self::ListSplit::*;

        let home = Rc::new(HomeView::default());
        let ignore = utils::get_ignored_shows()?;
        let episodes = dbqueries::get_episodes_widgets_filter_limit(&ignore, 100)?;
        let now_utc = Utc::now();

        let home_weak = Rc::downgrade(&home);
        let func = move |ep: EpisodeWidgetModel| {
            let home = match home_weak.upgrade() {
                Some(h) => h,
                None => return,
            };

            let epoch = ep.epoch();
            let widget = HomeEpisode::new(ep, &sender);

            match split(&now_utc, i64::from(epoch)) {
                Today => add_to_box(&widget, &home.today_list, &home.today_box),
                Yday => add_to_box(&widget, &home.yday_list, &home.yday_box),
                Week => add_to_box(&widget, &home.week_list, &home.week_box),
                Month => add_to_box(&widget, &home.month_list, &home.month_box),
                Rest => add_to_box(&widget, &home.rest_list, &home.rest_box),
            }
        };

        let callback = clone!(@weak home => @default-return (), move || {
            if let Some(ref v) = vadj {
                home.view.set_adjustments(None, Some(v))
            };
        });

        lazy_load_full(episodes, func, callback);
        Ok(home)
    }
}

fn add_to_box(widget: &HomeEpisode, listbox: &gtk::ListBox, box_: &gtk::Box) {
    listbox.append(&widget.row);
    box_.show();
}

fn split(now: &DateTime<Utc>, epoch: i64) -> ListSplit {
    let ep = Utc.timestamp(epoch, 0);
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
    container: gtk::Box,
    image: gtk::Image,
    episode: gtk::Box,
}

impl Default for HomeEpisode {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/home_episode.ui");
        let container: gtk::Box = builder.object("container").unwrap();
        let image: gtk::Image = builder.object("cover").unwrap();
        let ep = EpisodeWidget::default();
        container.append(&ep.container);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&container));
        row.show();

        HomeEpisode {
            row,
            container,
            image,
            episode: ep.container,
        }
    }
}

impl HomeEpisode {
    fn new(episode: EpisodeWidgetModel, sender: &Sender<Action>) -> HomeEpisode {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/home_episode.ui");
        let container: gtk::Box = builder.object("container").unwrap();
        let image: gtk::Image = builder.object("cover").unwrap();
        let pid = episode.show_id();
        let id = episode.rowid();
        let ep = EpisodeWidget::new(episode, sender);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&container));
        row.set_action_name(Some("app.go-to-episode"));
        row.set_action_target_value(Some(&id.to_variant()));
        row.show();

        let view = HomeEpisode {
            row,
            container,
            image,
            episode: ep.container.clone(),
        };

        view.init(pid);
        view
    }

    fn init(&self, show_id: i32) {
        self.set_cover(show_id);
        self.container.append(&self.episode);
    }

    fn set_cover(&self, show_id: i32) {
        utils::set_image_from_path(&self.image, show_id, 64)
            .map_err(|err| error!("Failed to set a cover: {}", err))
            .ok();
    }
}

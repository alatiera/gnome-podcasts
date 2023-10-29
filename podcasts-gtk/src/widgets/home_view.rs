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
use futures::future::JoinAll;

use adw::subclass::prelude::*;
use glib::subclass::InitializingObject;
use glib::Sender;
use gtk::gio;
use gtk::{glib, prelude::*, CompositeTemplate};

use podcasts_data::dbqueries;
use podcasts_data::EpisodeWidgetModel;

use crate::app::Action;
use crate::utils::{self, lazy_load};
use crate::widgets::{BaseView, EpisodeWidget};

#[derive(Debug, Clone)]
enum ListSplit {
    Today,
    Yday,
    Week,
    Month,
    Rest,
}

#[derive(Debug, Clone)]
struct DateBox(ListSplit, Vec<EpisodeWidgetModel>);

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
    pub(crate) fn new(sender: Sender<Action>) -> Result<Self> {
        let home: Self = glib::Object::new();

        crate::MAINCONTEXT.spawn_local_with_priority(
            glib::source::Priority::DEFAULT_IDLE,
            glib::clone!(@weak home => async move {
                let _ = home.add_to_boxes(sender).await;
            }),
        );

        Ok(home)
    }

    // FIMXE: there has to be a way to flatten the handes here
    async fn add_to_boxes(&self, sender: Sender<Action>) -> JoinAll<JoinAll<glib::JoinHandle<()>>> {
        let data = gio::spawn_blocking(get_episodes).await;

        let mut handles = Vec::with_capacity(5);
        if let Ok(Ok(data)) = data {
            for datebox in data {
                if datebox.1.is_empty() {
                    continue;
                }

                let handle = self.add_to_box(datebox, &sender).await;
                handles.push(handle);
            }
        }

        futures::future::join_all(handles)
    }

    async fn add_to_box(
        &self,
        datebox: DateBox,
        sender: &Sender<Action>,
    ) -> JoinAll<glib::JoinHandle<()>> {
        use self::ListSplit::*;

        let DateBox(date, model) = datebox;

        let box_ = match &date {
            Today => &self.imp().today_box,
            Yday => &self.imp().yday_box,
            Week => &self.imp().week_box,
            Month => &self.imp().month_box,
            Rest => &self.imp().rest_box,
        };

        let list = match &date {
            Today => &self.imp().today_list,
            Yday => &self.imp().yday_list,
            Week => &self.imp().week_list,
            Month => &self.imp().month_list,
            Rest => &self.imp().rest_list,
        };

        box_.set_visible(true);

        let sender = sender.clone();
        let constructor = move |ep: EpisodeWidgetModel| HomeEpisode::new(&sender, &ep).upcast();
        let list = list.upcast_ref::<gtk::Widget>().downgrade();
        lazy_load(model, list, constructor.clone()).await
    }
}

fn get_episodes() -> Result<Vec<DateBox>> {
    let ignore = utils::get_ignored_shows()?;
    let episodes = dbqueries::get_episodes_widgets_filter_limit(&ignore, 100)?;
    Ok(split_model(episodes))
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

fn split_model(model: Vec<EpisodeWidgetModel>) -> Vec<DateBox> {
    use self::ListSplit::*;

    let now_utc = Utc::now();

    let (mut today, mut yday, mut week, mut month, mut rest) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());

    for ep in model {
        let epoch = ep.epoch();
        match split(&now_utc, i64::from(epoch)) {
            Today => today.push(ep),
            Yday => yday.push(ep),
            Week => week.push(ep),
            Month => month.push(ep),
            Rest => rest.push(ep),
        }
    }

    vec![
        DateBox(Today, today),
        DateBox(Yday, yday),
        DateBox(Week, week),
        DateBox(Month, month),
        DateBox(Rest, rest),
    ]
}

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/home_episode.ui")]
pub struct HomeEpisodePriv {
    #[template_child]
    cover: TemplateChild<gtk::Image>,
    #[template_child]
    episode: TemplateChild<EpisodeWidget>,
}

impl HomeEpisodePriv {
    fn init(&self, sender: &Sender<Action>, episode: &EpisodeWidgetModel) {
        let pid = episode.show_id();
        self.set_cover(pid);
        self.episode.init(sender, episode);
        // Assure the image is read out along with the Episode title
        self.cover.set_accessible_role(gtk::AccessibleRole::Label);
    }

    fn set_cover(&self, show_id: i32) {
        if let Err(err) = utils::set_image_from_path(&self.cover, show_id, 64) {
            error!("Failed to set a cover: {err}");
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for HomeEpisodePriv {
    const NAME: &'static str = "PdHomeEpisode";
    type Type = HomeEpisode;
    type ParentType = gtk::ListBoxRow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for HomeEpisodePriv {}
impl ObjectImpl for HomeEpisodePriv {}
impl ListBoxRowImpl for HomeEpisodePriv {}

glib::wrapper! {
    pub struct HomeEpisode(ObjectSubclass<HomeEpisodePriv>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl HomeEpisode {
    pub(crate) fn new(sender: &Sender<Action>, episode: &EpisodeWidgetModel) -> Self {
        let widget = Self::default();
        widget.set_action_name(Some("app.go-to-episode"));
        widget.set_action_target_value(Some(&episode.rowid().to_variant()));
        widget.imp().init(sender, episode);
        widget
    }
}

impl Default for HomeEpisode {
    fn default() -> Self {
        let widget: Self = glib::Object::new();
        widget
    }
}

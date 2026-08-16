// queue_view.rs
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

use adw::subclass::prelude::*;
use async_channel::Sender;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, glib, prelude::*};

use crate::app::Action;
use crate::utils::lazy_load;
use crate::widgets::{BaseView, EpisodeWidget};
use podcasts_data::{EpisodeId, EpisodeModel, EpisodeWidgetModel};
use podcasts_data::{QueueItem, dbqueries};

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/queue_view.ui")]
pub struct QueueViewPriv {
    #[template_child]
    view: TemplateChild<BaseView>,
    #[template_child]
    queue_list: TemplateChild<gtk::ListBox>,
    #[template_child]
    empty_queue_status_page: TemplateChild<adw::StatusPage>,
}

#[glib::object_subclass]
impl ObjectSubclass for QueueViewPriv {
    const NAME: &'static str = "PdQueueView";
    type Type = QueueView;
    type ParentType = adw::Bin;

    fn class_init(klass: &mut Self::Class) {
        BaseView::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for QueueViewPriv {}
impl ObjectImpl for QueueViewPriv {}
impl BinImpl for QueueViewPriv {}

glib::wrapper! {
    pub struct QueueView(ObjectSubclass<QueueViewPriv>)
        @extends BaseView, gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl QueueView {
    pub(crate) fn new(sender: Sender<Action>) -> Self {
        let queue: Self = glib::Object::new();

        crate::MAINCONTEXT.spawn_local_with_priority(
            glib::source::Priority::DEFAULT_IDLE,
            glib::clone!(
                #[weak]
                queue,
                async move {
                    match dbqueries::get_queue() {
                        Ok(queue_items) => {
                            if queue_items.is_empty() {
                                queue.enter_empty_state();
                            } else {
                                queue.imp().queue_list.set_visible(true);
                                queue.imp().empty_queue_status_page.set_visible(false);
                                let constructor = move |queue_item: QueueItem| {
                                    let ep = EpisodeWidgetModel::from(
                                        dbqueries::get_episode_from_id(queue_item.episode_id())
                                            .unwrap(),
                                    );
                                    EpisodeWidget::new(&sender, ep, true, true).upcast()
                                };
                                let list = queue
                                    .imp()
                                    .queue_list
                                    .upcast_ref::<gtk::Widget>()
                                    .downgrade();
                                lazy_load(queue_items, list, constructor.clone()).await;
                            }
                        }
                        Err(e) => {
                            queue.enter_empty_state();
                            error!("Error could not create queue: {:?}", e);
                        }
                    }
                }
            ),
        );

        queue
    }

    pub(crate) fn get_episode_widget(&self, ep_id: EpisodeId) -> Option<(i32, EpisodeWidget)> {
        let mut i = 0;
        while let Some(row) = self.imp().queue_list.row_at_index(i) {
            if let Ok(queue_episode) = row.downcast::<EpisodeWidget>()
                && queue_episode.id() == ep_id
            {
                return Some((i, queue_episode));
            }
            i += 1;
        }
        None
    }

    pub(crate) fn update_episode(&self, ep: &EpisodeWidgetModel) {
        if let Some((_, queue_episode)) = self.get_episode_widget(ep.id()) {
            queue_episode.update_episode_state(ep);
        }
    }

    pub(crate) fn update_queue_after_move(
        &self,
        moved_episode: EpisodeId,
        new_index: usize,
    ) -> bool {
        let Some((_, moved_row)) = self.get_episode_widget(moved_episode) else {
            info!("Episode not on page, hard reload of Queue page.");
            return false;
        };

        self.imp().queue_list.remove(&moved_row);
        self.imp().queue_list.insert(&moved_row, new_index as i32);
        // preserve focus
        moved_row.grab_focus();

        // Return true if the update was successful
        true
    }

    pub(crate) fn update_queue_after_removal(&self, removed_episode: EpisodeId) -> bool {
        let Some((index, removed_row)) = self.get_episode_widget(removed_episode) else {
            info!("Episode not on page, hard reload of Queue page.");
            return false;
        };

        self.imp().queue_list.remove(&removed_row);
        // preserve focus on new item in position, if it was the last, focus the new last
        if let Some(new_focus) = self
            .imp()
            .queue_list
            .row_at_index(index)
            .or_else(|| self.imp().queue_list.row_at_index(index - 1))
        {
            info!("grabbing new focus");
            new_focus.grab_focus();
        } else {
            info!("no focus preserve");
        }

        if self.imp().queue_list.row_at_index(0).is_none() {
            self.enter_empty_state();
        }

        // Return true if the update was successful
        true
    }

    fn enter_empty_state(&self) {
        self.imp().queue_list.set_visible(false);
        self.imp().empty_queue_status_page.set_visible(true);
    }
}

// show.rs
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

use glib::clone;
use glib::Sender;
use gtk::Adjustment;

use anyhow::Result;
use fragile::Fragile;
use gtk::gio;
use tokio::sync::oneshot::error::TryRecvError;

use podcasts_data::dbqueries;
use podcasts_data::EpisodeWidgetModel;
use podcasts_data::Show;

use crate::app::Action;
use crate::utils::{self, lazy_load};
use crate::widgets::{BaseView, EmptyShow, EpisodeWidget, ReadMoreLabel, ShowMenu};

use std::cell::Cell;
use std::sync::Arc;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::{prelude::*, CompositeTemplate};

#[derive(Debug, Default, CompositeTemplate)]
#[template(resource = "/org/gnome/Podcasts/gtk/show_widget.ui")]
pub struct ShowWidgetPriv {
    #[template_child]
    pub cover: TemplateChild<gtk::Image>,
    #[template_child]
    pub read_more_label: TemplateChild<ReadMoreLabel>,
    #[template_child]
    pub episodes: TemplateChild<gtk::ListBox>,
    #[template_child]
    pub(crate) view: TemplateChild<BaseView>,

    pub show_id: Cell<Option<i32>>,
}

impl ShowWidgetPriv {
    fn init(&self) {
        self.read_more_label.init();
        self.read_more_label
            .update_property(&[gtk::accessible::Property::Description(
                "Podcast Description",
            )]);
    }

    /// Set the description text.
    fn set_description(&self, text: &str) {
        let markup = html2text::from_read(text.as_bytes(), text.as_bytes().len());
        let markup = markup.trim();
        if !markup.is_empty() {
            self.read_more_label.set_label(markup);
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ShowWidgetPriv {
    const NAME: &'static str = "PdShowWidget";
    type Type = super::ShowWidget;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        Self::bind_template(klass);
        klass.set_layout_manager_type::<gtk::BinLayout>();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ShowWidgetPriv {
    fn dispose(&self) {
        self.view.unparent();
    }
}

impl WidgetImpl for ShowWidgetPriv {}

glib::wrapper! {
    pub struct ShowWidget(ObjectSubclass<ShowWidgetPriv>)
        @extends gtk::Widget;
}

impl Default for ShowWidget {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ShowWidget {
    pub(crate) fn new(
        pd: Arc<Show>,
        sender: Sender<Action>,
        vadj: Option<Adjustment>,
    ) -> ShowWidget {
        let widget = ShowWidget::default();
        widget.init(&pd);

        let menu = ShowMenu::new(&pd, &widget.imp().episodes, &sender);
        send!(sender, Action::InitSecondaryMenu(Fragile::new(menu.menu)));

        let res = populate_listbox(&widget, pd, sender, vadj);
        debug_assert!(res.is_ok());

        widget
    }

    pub(crate) fn init(&self, pd: &Arc<Show>) {
        let self_ = self.imp();
        self_.init();
        self_.set_description(pd.description());
        self_.show_id.set(Some(pd.id()));

        let res = self.set_cover(pd);

        debug_assert!(res.is_ok());
    }

    /// Set the show cover.
    fn set_cover(&self, pd: &Arc<Show>) -> Result<()> {
        utils::set_image_from_path(&self.imp().cover, pd.id())
    }

    pub(crate) fn show_id(&self) -> Option<i32> {
        self.imp().show_id.get()
    }

    pub(crate) fn view(&self) -> BaseView {
        self.imp().view.clone()
    }
}

/// Populate the listbox with the shows episodes.
fn populate_listbox(
    show: &ShowWidget,
    pd: Arc<Show>,
    sender: Sender<Action>,
    vadj: Option<Adjustment>,
) -> Result<()> {
    let count = dbqueries::get_pd_episodes_count(&pd)?;
    let show_ = show.imp();

    let (sender_, mut receiver) = tokio::sync::oneshot::channel();
    gio::spawn_blocking(clone!(@strong pd => move || {
        if let Ok(episodes) = dbqueries::get_pd_episodeswidgets(&pd) {
            // The receiver can be dropped if there's an early return
            // like on show without episodes for example.
            let _ = sender_.send(episodes);
        }
    }));

    if count == 0 {
        let empty = EmptyShow::default();
        show_.episodes.append(&empty);
        return Ok(());
    }

    let list_weak = show_.episodes.downgrade();

    glib::idle_add_local(
        glib::clone!(@weak show => @default-return glib::ControlFlow::Break, move || {
            let episodes = match receiver.try_recv() {
                Ok(e) => e,
                Err(TryRecvError::Empty) => return glib::ControlFlow::Continue,
                Err(TryRecvError::Closed) => return glib::ControlFlow::Break,
            };

            debug_assert!(episodes.len() as i64 == count);

            let constructor = clone!(@strong sender => move |ep: EpisodeWidgetModel| {
                let id = ep.rowid();
                let episode_widget = EpisodeWidget::new(&sender, &ep);
                let row = gtk::ListBoxRow::new();
                row.set_child(Some(&episode_widget));
                row.set_action_name(Some("app.go-to-episode"));
                row.set_action_target_value(Some(&id.to_variant()));
                row
            });

            let callback = clone!(@weak show, @strong vadj => move || {
                show.imp().view.set_adjustments(None, vadj.as_ref());
            });

            lazy_load(episodes, list_weak.clone(), constructor, callback);

            glib::ControlFlow::Break
        }),
    );

    Ok(())
}

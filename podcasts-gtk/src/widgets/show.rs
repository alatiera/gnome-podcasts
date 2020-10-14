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

use glib;
use glib::clone;
use glib::Sender;
use gtk::{self, prelude::*, Adjustment};

use anyhow::Result;
use crossbeam_channel::bounded;
use fragile::Fragile;
use html2text;
use libhandy::{Clamp, ClampExt};
use rayon;

use podcasts_data::dbqueries;
use podcasts_data::Show;

use crate::app::Action;
use crate::utils::{self, lazy_load};
use crate::widgets::{BaseView, EmptyShow, EpisodeWidget, ShowMenu};

use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct ShowWidget {
    pub(crate) view: BaseView,
    cover: gtk::Image,
    description: gtk::Label,
    description_short: gtk::Label,
    description_stack: gtk::Stack,
    description_button: gtk::Button,
    description_button_revealer: gtk::Revealer,
    episodes: gtk::ListBox,
    show_id: Option<i32>,
}

impl Default for ShowWidget {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/show_widget.ui");
        let sub_cont: gtk::Box = builder.get_object("sub_container").unwrap();
        let cover: gtk::Image = builder.get_object("cover").unwrap();
        let description: gtk::Label = builder.get_object("description").unwrap();
        let description_short: gtk::Label = builder.get_object("description_short").unwrap();
        let description_stack: gtk::Stack = builder.get_object("description_stack").unwrap();
        let description_button: gtk::Button = builder.get_object("description_button").unwrap();
        let description_button_revealer =
            builder.get_object("description_button_revealer").unwrap();
        let episodes: gtk::ListBox = builder.get_object("episodes").unwrap();
        let view = BaseView::default();

        let clamp = Clamp::new();
        clamp.set_maximum_size(700);

        clamp.add(&sub_cont);
        view.add(&clamp);
        clamp.show_all();

        ShowWidget {
            view,
            cover,
            description,
            description_short,
            description_stack,
            description_button,
            description_button_revealer,
            episodes,
            show_id: None,
        }
    }
}

impl ShowWidget {
    pub(crate) fn new(
        pd: Arc<Show>,
        sender: Sender<Action>,
        vadj: Option<Adjustment>,
    ) -> Rc<ShowWidget> {
        let mut pdw = ShowWidget::default();
        pdw.init(&pd);

        let menu = ShowMenu::new(&pd, &pdw.episodes, &sender);
        send!(sender, Action::InitShowMenu(Fragile::new(menu)));

        let pdw = Rc::new(pdw);
        let res = populate_listbox(&pdw, pd.clone(), sender, vadj);
        debug_assert!(res.is_ok());

        pdw.description_short
            .connect_size_allocate(clone!(@weak pdw => move |_, _2| {
                pdw.update_read_more();
            }));

        pdw.description_button
            .connect_clicked(clone!(@weak pdw => move |_| {
                pdw.description_stack.set_visible_child_name("full");
            }));

        pdw
    }

    pub(crate) fn init(&mut self, pd: &Arc<Show>) {
        self.set_description(pd.description());
        self.show_id = Some(pd.id());

        let res = self.set_cover(&pd);
        debug_assert!(res.is_ok());
    }

    /// Set the show cover.
    fn set_cover(&self, pd: &Arc<Show>) -> Result<()> {
        utils::set_image_from_path(&self.cover, pd.id(), 256)
    }

    fn update_read_more(&self) {
        if let Some(layout) = self.description_short.get_layout() {
            let more = layout.is_ellipsized()
                || self.description.get_label() != self.description_short.get_label();
            self.description_button_revealer.set_reveal_child(more);
        }
    }

    /// Set the description text.
    fn set_description(&self, text: &str) {
        let markup = html2text::from_read(text.as_bytes(), text.as_bytes().len());
        let markup = markup.trim();
        let lines: Vec<&str> = markup.lines().collect();

        if markup.is_empty() {
            self.description_stack.set_visible(false);
        } else {
            self.description_stack.set_visible(true);

            self.description.set_markup(markup);
            debug_assert!(lines.len() > 0);
            if lines.len() > 0 {
                self.description_short.set_markup(lines[0]);
            }
        }
    }

    pub(crate) fn show_id(&self) -> Option<i32> {
        self.show_id
    }
}

/// Populate the listbox with the shows episodes.
fn populate_listbox(
    show: &Rc<ShowWidget>,
    pd: Arc<Show>,
    sender: Sender<Action>,
    vadj: Option<Adjustment>,
) -> Result<()> {
    use crossbeam_channel::TryRecvError;

    let count = dbqueries::get_pd_episodes_count(&pd)?;

    let (sender_, receiver) = bounded(1);
    rayon::spawn(clone!(@strong pd => move || {
        if let Ok(episodes) = dbqueries::get_pd_episodeswidgets(&pd) {
            // The receiver can be dropped if there's an early return
            // like on show without episodes for example.
            let _ = sender_.send(episodes);
        }
    }));

    if count == 0 {
        let empty = EmptyShow::default();
        show.episodes.add(empty.deref());
        return Ok(());
    }

    let show_weak = Rc::downgrade(&show);
    let list_weak = show.episodes.downgrade();
    gtk::idle_add(move || {
        let episodes = match receiver.try_recv() {
            Ok(e) => e,
            Err(TryRecvError::Empty) => return glib::Continue(true),
            Err(TryRecvError::Disconnected) => return glib::Continue(false),
        };

        debug_assert!(episodes.len() as i64 == count);

        let constructor = clone!(@strong sender => move |ep| {
            EpisodeWidget::new(ep, &sender).container.clone()
        });

        let callback = clone!(@strong show_weak, @strong vadj => move || {
            match (show_weak.upgrade(), &vadj) {
                (Some(ref shows), Some(ref v)) => shows.view.set_adjustments(None, Some(v)),
                _ => (),
            };
        });

        lazy_load(episodes, list_weak.clone(), constructor, callback);

        glib::Continue(false)
    });

    Ok(())
}

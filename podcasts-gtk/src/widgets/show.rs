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
use gtk::{self, prelude::*, Adjustment};

use crossbeam_channel::{bounded, Sender};
use failure::Error;
use fragile::Fragile;
use html2text;
use libhandy::{Column, ColumnExt};
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
    episodes: gtk::ListBox,
    show_id: Option<i32>,
}

impl Default for ShowWidget {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/show_widget.ui");
        let sub_cont: gtk::Box = builder.get_object("sub_container").unwrap();
        let cover: gtk::Image = builder.get_object("cover").unwrap();
        let description: gtk::Label = builder.get_object("description").unwrap();
        let episodes = builder.get_object("episodes").unwrap();
        let view = BaseView::default();

        let column = Column::new();
        column.set_maximum_width(700);
        // For some reason the Column is not seen as a gtk::container
        // and therefore we can't call add() without the cast
        let column = column.upcast::<gtk::Widget>();
        let column = column.downcast::<gtk::Container>().unwrap();

        column.add(&sub_cont);
        view.add(&column);
        column.show_all();

        ShowWidget {
            view,
            cover,
            description,
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
        sender.send(Action::InitShowMenu(Fragile::new(menu)));

        let pdw = Rc::new(pdw);
        let res = populate_listbox(&pdw, pd.clone(), sender, vadj);
        debug_assert!(res.is_ok());

        pdw
    }

    pub(crate) fn init(&mut self, pd: &Arc<Show>) {
        self.set_description(pd.description());
        self.show_id = Some(pd.id());

        let res = self.set_cover(&pd);
        debug_assert!(res.is_ok());
    }

    /// Set the show cover.
    fn set_cover(&self, pd: &Arc<Show>) -> Result<(), Error> {
        utils::set_image_from_path(&self.cover, pd.id(), 256)
    }

    /// Set the description text.
    fn set_description(&self, text: &str) {
        self.description
            .set_markup(html2text::from_read(text.as_bytes(), 80).trim());
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
) -> Result<(), Error> {
    let count = dbqueries::get_pd_episodes_count(&pd)?;

    let (sender_, receiver) = bounded(1);
    rayon::spawn(clone!(pd => move || {
        if let Ok(episodes) = dbqueries::get_pd_episodeswidgets(&pd) {
            // The receiver can be dropped if there's an early return
            // like on show without episodes for example.
            sender_.send(episodes);
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
            Some(e) => e,
            None => return glib::Continue(true),
        };
        debug_assert!(episodes.len() as i64 == count);

        let constructor = clone!(sender => move |ep| {
            EpisodeWidget::new(ep, &sender).container.clone()
        });

        let callback = clone!(show_weak, vadj => move || {
            match (show_weak.upgrade(), &vadj) {
                (Some(ref shows), Some(ref v)) => shows.view.set_adjutments(None, Some(v)),
                _ => (),
            };
        });

        lazy_load(episodes, list_weak.clone(), constructor, callback);

        glib::Continue(false)
    });

    Ok(())
}

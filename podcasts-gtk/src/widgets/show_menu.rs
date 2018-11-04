// show_menu.rs
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
use gtk;
use gtk::prelude::*;

use crossbeam_channel::Sender;
use failure::Error;
use open;
use rayon;

use podcasts_data::dbqueries;
use podcasts_data::utils::delete_show;
use podcasts_data::Show;

use app::Action;
use utils;
use widgets::appnotif::InAppNotification;

use std::sync::Arc;

use i18n::{i18n, i18n_f};

#[derive(Debug, Clone)]
pub(crate) struct ShowMenu {
    pub(crate) container: gtk::PopoverMenu,
    website: gtk::ModelButton,
    played: gtk::ModelButton,
    unsub: gtk::ModelButton,
}

impl Default for ShowMenu {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/show_menu.ui");
        let container = builder.get_object("menu").unwrap();
        let website = builder.get_object("website").unwrap();
        let played = builder.get_object("played").unwrap();
        let unsub = builder.get_object("unsub").unwrap();

        ShowMenu {
            container,
            website,
            played,
            unsub,
        }
    }
}

impl ShowMenu {
    pub(crate) fn new(pd: &Arc<Show>, episodes: &gtk::ListBox, sender: &Sender<Action>) -> Self {
        let s = Self::default();
        s.init(pd, episodes, sender);
        s
    }

    fn init(&self, pd: &Arc<Show>, episodes: &gtk::ListBox, sender: &Sender<Action>) {
        self.connect_website(pd);
        self.connect_played(pd, episodes, sender);
        self.connect_unsub(pd, sender)
    }

    fn connect_website(&self, pd: &Arc<Show>) {
        self.website.set_tooltip_text(Some(pd.link()));
        self.website.connect_clicked(clone!(pd => move |_| {
            let link = pd.link();
            info!("Opening link: {}", link);
            let res = open::that(link);
            debug_assert!(res.is_ok());
        }));
    }

    fn connect_played(&self, pd: &Arc<Show>, episodes: &gtk::ListBox, sender: &Sender<Action>) {
        let episodes_weak = episodes.downgrade();
        self.played.connect_clicked(clone!(pd, sender => move |_| {
            let episodes = match episodes_weak.upgrade() {
                Some(e) => e,
                None => return,
            };

            let res = dim_titles(&episodes);
            debug_assert!(res.is_some());

            sender.send(Action::MarkAllPlayerNotification(pd.clone()))
        }));
    }

    fn connect_unsub(&self, pd: &Arc<Show>, sender: &Sender<Action>) {
        self.unsub
            .connect_clicked(clone!(pd, sender => move |unsub| {
            // hack to get away without properly checking for none.
            // if pressed twice would panic.
            unsub.set_sensitive(false);

            sender.send(Action::RemoveShow(pd.clone()));

            sender.send(Action::HeaderBarNormal);
            sender.send(Action::ShowShowsAnimated);
            // Queue a refresh after the switch to avoid blocking the db.
            sender.send(Action::RefreshShowsView);
            sender.send(Action::RefreshEpisodesView);

            unsub.set_sensitive(true);
        }));
    }
}

// Ideally if we had a custom widget this would have been as simple as:
// `for row in listbox { ep = row.get_episode(); ep.dim_title(); }`
// But now I can't think of a better way to do it than hardcoding the title
// position relative to the EpisodeWidget container gtk::Box.
fn dim_titles(episodes: &gtk::ListBox) -> Option<()> {
    let children = episodes.get_children();

    for row in children {
        let row = row.downcast::<gtk::ListBoxRow>().ok()?;
        let container = row.get_children().remove(0).downcast::<gtk::Box>().ok()?;
        let foo = container
            .get_children()
            .remove(0)
            .downcast::<gtk::Box>()
            .ok()?;
        let bar = foo.get_children().remove(0).downcast::<gtk::Box>().ok()?;
        let title = bar.get_children().remove(0).downcast::<gtk::Label>().ok()?;

        title.get_style_context().map(|c| c.add_class("dim-label"));
    }
    Some(())
}

fn mark_all_watched(pd: &Show, sender: &Sender<Action>) -> Result<(), Error> {
    // TODO: If this fails for whatever reason, it should be impossible, show an error
    dbqueries::update_none_to_played_now(pd)?;
    // Not all widgets might have been loaded when the mark_all is hit
    // So we will need to refresh again after it's done.
    sender.send(Action::RefreshWidgetIfSame(pd.id()));
    sender.send(Action::RefreshEpisodesView);
    Ok(())
}

pub(crate) fn mark_all_notif(pd: Arc<Show>, sender: &Sender<Action>) -> InAppNotification {
    let id = pd.id();
    let sender_ = sender.clone();
    let callback = move |revealer: gtk::Revealer| {
        let res = mark_all_watched(&pd, &sender_);
        debug_assert!(res.is_ok());

        revealer.set_reveal_child(false);
        glib::Continue(false)
    };

    let undo_callback = clone!(sender => move || sender.send(Action::RefreshWidgetIfSame(id)));
    let text = i18n("Marked all episodes as listened");
    InAppNotification::new(&text, 6000, callback, Some(undo_callback))
}

pub(crate) fn remove_show_notif(pd: Arc<Show>, sender: Sender<Action>) -> InAppNotification {
    let text = i18n_f("Unsubscribed from {}", &[pd.title()]);

    let res = utils::ignore_show(pd.id());
    debug_assert!(res.is_ok());

    let sender_ = sender.clone();
    let pd_ = pd.clone();
    let callback = move |revealer: gtk::Revealer| {
        let res = utils::uningore_show(pd_.id());
        debug_assert!(res.is_ok());

        // Spawn a thread so it won't block the ui.
        rayon::spawn(clone!(pd_, sender_ => move || {
            delete_show(&pd_)
                .map_err(|err| error!("Error: {}", err))
                .map_err(|_| error!("Failed to delete {}", pd_.title()))
                .ok();

            sender_.send(Action::RefreshEpisodesView);
        }));

        revealer.set_reveal_child(false);
        glib::Continue(false)
    };

    let undo_callback = move || {
        let res = utils::uningore_show(pd.id());
        debug_assert!(res.is_ok());
        sender.send(Action::RefreshShowsView);
        sender.send(Action::RefreshEpisodesView);
    };

    InAppNotification::new(&text, 6000, callback, Some(undo_callback))
}

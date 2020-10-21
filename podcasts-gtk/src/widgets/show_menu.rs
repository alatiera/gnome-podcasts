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

use gio::ActionMapExt;
use glib;
use glib::clone;
use gtk;
use gtk::prelude::*;

use anyhow::Result;
use glib::Sender;
use open;
use rayon;

use podcasts_data::dbqueries;
use podcasts_data::utils::delete_show;
use podcasts_data::Show;

use crate::app::Action;
use crate::utils;
use crate::widgets::appnotif::InAppNotification;

use std::sync::Arc;

use crate::i18n::{i18n, i18n_f};

#[derive(Debug, Clone)]
pub(crate) struct ShowMenu {
    pub(crate) menu: gio::MenuModel,
    website: gio::SimpleAction,
    played: gio::SimpleAction,
    unsub: gio::SimpleAction,
    group: gio::SimpleActionGroup,
}

impl Default for ShowMenu {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/show_menu.ui");
        let menu = builder.get_object("show_menu").unwrap();
        let website = gio::SimpleAction::new("open-website", None);
        let played = gio::SimpleAction::new("mark-played", None);
        let unsub = gio::SimpleAction::new("unsubscribe", None);
        let group = gio::SimpleActionGroup::new();

        group.add_action(&website);
        group.add_action(&played);
        group.add_action(&unsub);

        ShowMenu {
            menu,
            website,
            played,
            unsub,
            group,
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
        self.connect_unsub(pd, sender);

        let app = gio::Application::get_default()
            .expect("Could not get default application")
            .downcast::<gtk::Application>()
            .unwrap();
        let win = app.get_active_window().expect("No active window");
        win.insert_action_group("show", Some(&self.group));
    }

    fn connect_website(&self, pd: &Arc<Show>) {
        // TODO: tooltips for actions?
        self.website
            .connect_activate(clone!(@strong pd => move |_, _| {
                let link = pd.link();
                info!("Opening link: {}", link);
                let res = open::that(link);
                debug_assert!(res.is_ok());
            }));
    }

    fn connect_played(&self, pd: &Arc<Show>, episodes: &gtk::ListBox, sender: &Sender<Action>) {
        self.played.connect_activate(
            clone!(@strong pd, @strong sender, @weak episodes => move |_, _| {
                let res = dim_titles(&episodes);
                debug_assert!(res.is_some());

                send!(sender, Action::MarkAllPlayerNotification(pd.clone()));
            }),
        );
    }

    fn connect_unsub(&self, pd: &Arc<Show>, sender: &Sender<Action>) {
        self.unsub
            .connect_activate(clone!(@strong pd, @strong sender => move |unsub, _| {
                unsub.set_enabled(false);

                send!(sender, Action::RemoveShow(pd.clone()));

                send!(sender, Action::HeaderBarNormal);
                send!(sender, Action::ShowShowsAnimated);
                // Queue a refresh after the switch to avoid blocking the db.
                send!(sender, Action::RefreshShowsView);
                send!(sender, Action::RefreshEpisodesView);

                unsub.set_enabled(true);
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
        let baz = bar.get_children().remove(0).downcast::<gtk::Box>().ok()?;
        let title = baz.get_children().remove(0).downcast::<gtk::Label>().ok()?;

        title.get_style_context().add_class("dim-label");

        let checkmark = baz.get_children().remove(1).downcast::<gtk::Image>().ok()?;
        checkmark.show();
    }
    Some(())
}

fn mark_all_watched(pd: &Show, sender: &Sender<Action>) -> Result<()> {
    // TODO: If this fails for whatever reason, it should be impossible, show an error
    dbqueries::update_none_to_played_now(pd)?;
    // Not all widgets might have been loaded when the mark_all is hit
    // So we will need to refresh again after it's done.
    send!(sender, Action::RefreshWidgetIfSame(pd.id()));
    send!(sender, Action::RefreshEpisodesView);
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

    let undo_callback = clone!(@strong sender => move || {
        send!(sender, Action::RefreshWidgetIfSame(id));
    });
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
        let res = utils::unignore_show(pd_.id());
        debug_assert!(res.is_ok());

        // Spawn a thread so it won't block the ui.
        rayon::spawn(clone!(@strong pd_, @strong sender_ => move || {
            delete_show(&pd_)
                .map_err(|err| error!("Error: {}", err))
                .map_err(|_| error!("Failed to delete {}", pd_.title()))
                .ok();

            send!(sender_, Action::RefreshEpisodesView);
        }));

        revealer.set_reveal_child(false);
        glib::Continue(false)
    };

    let undo_callback = move || {
        let res = utils::unignore_show(pd.id());
        debug_assert!(res.is_ok());
        send!(sender, Action::RefreshShowsView);
        send!(sender, Action::RefreshEpisodesView);
    };

    InAppNotification::new(&text, 6000, callback, Some(undo_callback))
}

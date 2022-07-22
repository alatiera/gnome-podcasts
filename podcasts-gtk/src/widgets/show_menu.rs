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

use gio::prelude::ActionMapExt;
use glib::clone;
use gtk::prelude::*;

use anyhow::Result;
use glib::Sender;

use podcasts_data::dbqueries;
use podcasts_data::utils::delete_show;
use podcasts_data::Show;

use crate::app::Action;
use crate::utils;

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
        let menu = builder.object("show_menu").unwrap();
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

        let app = gio::Application::default()
            .expect("Could not get default application")
            .downcast::<gtk::Application>()
            .unwrap();
        let win = app.active_window().expect("No active window");
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
    // FIXME This api should only be used for widget implementations.
    let listmodel = episodes.observe_children();
    for i in 0..listmodel.n_items() {
        let obj = listmodel.item(i)?;
        let row = obj.downcast_ref::<gtk::ListBoxRow>()?;
        dim_row_title(&row)?;
    }
    Some(())
}

fn dim_row_title(row: &gtk::ListBoxRow) -> Option<()> {
    // FIXME first_child should only be used for widget implementations.
    let container = row.first_child()?.downcast::<gtk::Box>().ok()?;
    let foo = container.first_child()?.downcast::<gtk::Box>().ok()?;
    let bar = foo.first_child()?.downcast::<gtk::Box>().ok()?;
    let baz = bar.first_child()?.downcast::<gtk::Box>().ok()?;
    let title = baz.first_child()?.downcast::<gtk::Label>().ok()?;

    title.add_css_class("dim-label");

    // FIXME next_sibling should only be used for widget implementations.
    let checkmark = title.next_sibling()?.downcast::<gtk::Image>().ok()?;
    checkmark.show();
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

pub(crate) fn mark_all_notif(pd: Arc<Show>, sender: &Sender<Action>) -> adw::Toast {
    let id = pd.id();
    let toast = adw::Toast::new(&i18n("Marked all episodes as listened"));
    toast.set_button_label(Some(&i18n("Undo")));
    toast.set_action_target_value(Some(&id.to_variant()));
    toast.set_action_name(Some("app.undo-mark-all"));

    toast.connect_dismissed(clone!(@strong sender => move |_| {
        let app = gio::Application::default()
            .expect("Could not get default application")
            .downcast::<crate::PdApplication>()
            .unwrap();
        if app.is_show_marked_mark(&pd) {
            let res = mark_all_watched(&pd, &sender);
            debug_assert!(res.is_ok());
        }
    }));

    toast
}

pub(crate) fn remove_show_notif(pd: Arc<Show>, sender: Sender<Action>) -> adw::Toast {
    let text = i18n_f("Unsubscribed from {}", &[pd.title()]);
    let id = pd.id();

    let toast = adw::Toast::new(&text);
    toast.set_button_label(Some(&i18n("Undo")));
    toast.set_action_target_value(Some(&id.to_variant()));
    toast.set_action_name(Some("app.undo-remove-show"));

    let res = utils::ignore_show(id);
    debug_assert!(res.is_ok());

    toast.connect_dismissed(clone!(@strong sender => move |_args| {
        let res = utils::unignore_show(id);
        debug_assert!(res.is_ok());

        // Spawn a thread so it won't block the ui.
        rayon::spawn(clone!(@strong pd, @strong sender => move || {
            let app = gio::Application::default()
                .expect("Could not get default application")
                .downcast::<crate::PdApplication>()
                .unwrap();
            if app.is_show_marked_delete(&pd) {
                delete_show(&pd)
                    .map_err(|err| error!("Error: {}", err))
                    .map_err(|_| error!("Failed to delete {}", pd.title()))
                    .ok();

                send!(sender, Action::RefreshEpisodesView);
            }
        }));
    }));

    toast
}

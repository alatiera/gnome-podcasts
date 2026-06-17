// show_menu.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
// Copyright 2023-2026 nee <nee-git@patchouli.garden>
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
use async_channel::Sender;
use formatx::formatx;
use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk::{gio, glib};
use std::sync::Arc;

use crate::app::Action;
use crate::utils;
use crate::widgets::ShowWidget;
use podcasts_data::Show;
use podcasts_data::dbqueries;
use podcasts_data::utils::delete_show;

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
    pub(crate) fn new(pd: &Arc<Show>, widget: &ShowWidget, sender: &Sender<Action>) -> Self {
        let s = Self::default();
        s.init(pd, widget, sender);
        s
    }

    fn init(&self, pd: &Arc<Show>, widget: &ShowWidget, sender: &Sender<Action>) {
        self.connect_website(pd);
        self.connect_played(pd, widget, sender);
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
        self.website.connect_activate(clone!(
            #[strong]
            pd,
            move |_, _| {
                let link = pd.link();
                info!("Opening link: {}", link);
                let res = open::that(link);
                debug_assert!(res.is_ok());
            }
        ));
    }

    fn connect_played(&self, pd: &Arc<Show>, widget: &ShowWidget, sender: &Sender<Action>) {
        self.played.connect_activate(clone!(
            #[strong]
            pd,
            #[strong]
            sender,
            #[weak]
            widget,
            move |_, _| {
                widget.mark_all_played();
                send_blocking!(sender, Action::MarkAllPlayerNotification(pd.clone()));
            }
        ));
    }

    fn connect_unsub(&self, pd: &Arc<Show>, sender: &Sender<Action>) {
        self.unsub.connect_activate(clone!(
            #[strong]
            pd,
            #[strong]
            sender,
            move |unsub, _| {
                unsub.set_enabled(false);
                send_blocking!(sender, Action::RemoveShow(pd.clone()));
                // Queue a refresh after the switch to avoid blocking the db.
                send_blocking!(sender, Action::RefreshShowsView);
                send_blocking!(sender, Action::RefreshEpisodesView);
                unsub.set_enabled(true);
            }
        ));
    }
}

fn mark_all_watched(pd: &Show, sender: &Sender<Action>) -> Result<()> {
    // TODO: If this fails for whatever reason, it should be impossible, show an error
    dbqueries::update_none_to_played_now(pd)?;
    // Not all widgets might have been loaded when the mark_all is hit
    // So we will need to refresh again after it's done.
    send_blocking!(sender, Action::RefreshWidgetIfSame(pd.id()));
    send_blocking!(sender, Action::RefreshEpisodesView);
    Ok(())
}

pub(crate) fn mark_all_notif(pd: Arc<Show>, sender: &Sender<Action>) -> adw::Toast {
    let id = pd.id();
    let toast = adw::Toast::new(&gettext("Marked all episodes as listened"));
    toast.set_button_label(Some(&gettext("Undo")));
    toast.set_action_target_value(Some(&id.0.to_variant()));
    toast.set_action_name(Some("app.undo-mark-all"));

    toast.connect_dismissed(clone!(
        #[strong]
        sender,
        move |_| {
            let app = gio::Application::default()
                .expect("Could not get default application")
                .downcast::<crate::PdApplication>()
                .unwrap();
            if app.is_show_marked_mark(&pd) {
                let res = mark_all_watched(&pd, &sender);
                debug_assert!(res.is_ok());
            }
        }
    ));

    toast
}

pub(crate) fn remove_show_notif(pd: Arc<Show>, sender: Sender<Action>) -> adw::Toast {
    let text = formatx!(gettext("Unsubscribed from {}"), pd.title())
        .expect("Could not format translatable string");
    let id = pd.id();

    let toast = adw::Toast::new(&text);
    toast.set_button_label(Some(&gettext("Undo")));
    toast.set_action_target_value(Some(&id.0.to_variant()));
    toast.set_action_name(Some("app.undo-remove-show"));

    let res = utils::ignore_show(id);
    debug_assert!(res.is_ok());

    toast.connect_dismissed(move |_args| {
        let res = utils::unignore_show(id);
        debug_assert!(res.is_ok());

        // Spawn a thread so it won't block the ui.
        gio::spawn_blocking(clone!(
            #[strong]
            pd,
            #[strong]
            sender,
            move || {
                let app = gio::Application::default()
                    .expect("Could not get default application")
                    .downcast::<crate::PdApplication>()
                    .unwrap();
                if app.is_show_marked_delete(&pd) {
                    if let Err(err) = podcasts_data::sync::Show::store(&pd, podcasts_data::sync::ShowAction::Removed) {
                        // TODO okay to send_blocking here?
                        send_blocking!(sender, Action::ErrorNotification(
                            formatx!(gettext("Failed to store Unsubscribe Action for Synchronization in database. {}"), err)
                                .expect("Could not format translatable string"))
                        )
                    }


                    if let Err(err) = delete_show(&pd) {
                        error!("Error: {}", err);
                        error!("Failed to delete {}", pd.title());
                    }

                    send_blocking!(sender, Action::QuickSyncNextcloud);
                    // No need to update the UI after remove.
                    // The "unsubscribe" action already updated the UI after ignoring the show.
                }
            }
        ));
    });

    toast
}

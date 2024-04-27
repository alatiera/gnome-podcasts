// populated.rs
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
use async_channel::Sender;
use glib::WeakRef;
use gtk::glib;
use gtk::prelude::*;
use std::sync::Arc;

use crate::app::Action;
use crate::widgets::{ShowWidget, ShowsView};
use crate::window::MainWindow;
use podcasts_data::dbqueries;
use podcasts_data::Show;

#[derive(Debug, Clone)]
pub(crate) struct PopulatedStack {
    container: gtk::Box,
    populated: ShowsView,
    show: ShowWidget,
    sender: Sender<Action>,
    window: Option<WeakRef<MainWindow>>,
}

impl PopulatedStack {
    pub(crate) fn new(sender: Sender<Action>) -> PopulatedStack {
        let populated = ShowsView::new(sender.clone());
        let show = ShowWidget::default();
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        container.append(populated.view());

        PopulatedStack {
            container,
            populated,
            show,
            sender,
            window: None,
        }
    }

    pub(crate) fn update(&mut self) {
        if let Err(err) = self.update_widget() {
            error!("Could not update widget: {err}");
        }
        if let Err(err) = self.update_shows() {
            error!("Could not update shows: {err}");
        }
    }

    pub(crate) fn update_shows(&mut self) -> Result<()> {
        let old = &self.populated.view().clone();
        debug!("Name: {:?}", old.widget_name());

        self.populated = ShowsView::new(self.sender.clone());

        self.container.remove(old);
        self.container.append(self.populated.view());

        Ok(())
    }

    pub(crate) fn replace_widget(&mut self, pd: Arc<Show>) -> Result<ShowWidget> {
        let title = pd.title().to_owned();
        let new = ShowWidget::new(pd, self.sender.clone());

        self.show = new.clone();

        if let Some(window) = self.window.as_ref().and_then(|w| w.upgrade()) {
            window.replace_show_widget(Some(&new), &title);
        }

        Ok(new)
    }

    pub(crate) fn set_window(&mut self, window: &MainWindow) {
        self.window = Some(window.downgrade())
    }

    pub(crate) fn update_widget(&mut self) -> Result<()> {
        let id = self.show.show_id();
        if id.is_none() {
            return Ok(());
        }

        let pd = dbqueries::get_podcast_from_id(id.unwrap_or_default())?;
        self.replace_widget(Arc::new(pd))?;

        Ok(())
    }

    // Only update widget if its show_id is equal to pid.
    pub(crate) fn update_widget_if_same(&mut self, pid: i32) -> Result<()> {
        if self.show.show_id() != Some(pid) {
            debug!("Different widget. Early return");
            return Ok(());
        }

        self.update_widget()
    }

    pub(crate) fn container(&self) -> gtk::Box {
        self.container.clone()
    }
}

// mod.rs
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

mod aboutdialog;
pub(crate) mod appnotif;
mod base_view;
mod empty;
mod episode;
mod home_view;
pub(crate) mod player;
mod show;
pub(crate) mod show_menu;
mod shows_view;

pub(crate) use self::aboutdialog::about_dialog;
pub(crate) use self::base_view::BaseView;
pub(crate) use self::empty::{EmptyShow, EmptyView};
pub(crate) use self::episode::EpisodeWidget;
pub(crate) use self::home_view::HomeView;
pub(crate) use self::show::ShowWidget;
pub(crate) use self::show_menu::ShowMenu;
pub(crate) use self::shows_view::ShowsView;

#[cfg(test)]
pub(crate) use self::home_view::HomeEpisode;

// queue_item.rs
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

use crate::EpisodeId;
use crate::make_id_wrapper;
use crate::schema::queue;

make_id_wrapper!(QueueId);

#[derive(Queryable, Identifiable, AsChangeset, PartialEq, Selectable)]
#[diesel(table_name = queue)]
#[diesel(treat_none_as_null = true)]
#[derive(Debug, Clone)]
/// Diesel Model of the queue table.
pub struct QueueItem {
    id: QueueId,
    episode_id: EpisodeId,
    position: f64,
}

impl QueueItem {
    /// Get the queue item `id` column.
    pub fn id(&self) -> QueueId {
        self.id
    }

    /// Get the id of the queued episode
    pub fn episode_id(&self) -> EpisodeId {
        self.episode_id
    }

    /// Get the position of the episode in the queue (episodes are played in ascending order of position)
    pub fn position(&self) -> f64 {
        self.position
    }
}

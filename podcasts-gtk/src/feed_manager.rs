// feed_manager.rs
//
// Copyright 2024 nee <nee-git@patchouli.garden>
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
use podcasts_data::Source;
use podcasts_data::dbqueries;
use podcasts_data::pipeline::pipeline;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use tokio::sync::watch;

use crate::app::Action;
use crate::glib::Priority;

pub(crate) static FEED_MANAGER: LazyLock<FeedManager> = LazyLock::new(FeedManager::default);

type RefreshId = u64;
#[derive(Debug)]
struct RefreshBatch {
    represents_full_refresh: bool,
    feeds: Vec<Source>,
    receiver: watch::Receiver<bool>,
}
#[derive(Debug, Default)]
struct State {
    next_id: RefreshId,
    currently_running: HashMap<RefreshId, RefreshBatch>,
}

#[derive(Debug, Default)]
pub struct FeedManager {
    state: RwLock<State>,
}

impl FeedManager {
    /// refresh all feeds, or waits for a running refresh to finish
    #[allow(dead_code)]
    pub async fn full_refresh(&self, sender: &Sender<Action>) {
        if let Some(mut refresh_done) = self.schedule_full_refresh(sender) {
            if let Err(e) = refresh_done.wait_for(|v| *v).await {
                error!("Failed to receive feed_manager {e}");
            }
        };
    }

    /// The non-async variant of full_refresh
    /// returns None when skipped due to empty database
    pub fn schedule_full_refresh(&self, sender: &Sender<Action>) -> Option<watch::Receiver<bool>> {
        // If we try to update the whole db, but the db is empty, exit early
        match dbqueries::is_source_populated(&[]) {
            Ok(false) => {
                info!("No feed sources in db, skipping refresh");
                return None;
            }
            Err(err) => error!("Failed to check for empty podcast DB: {err}"),
            _ => (),
        };

        let running_full_refresh = if let Ok(state) = self.state.read() {
            state
                .currently_running
                .iter()
                .find(|(_, v)| v.represents_full_refresh)
                .map(|(_, v)| v.receiver.clone())
        } else {
            error!("Couldn't lock feed_manager state to schedule_full_refresh");
            return None;
        };
        running_full_refresh.or_else(|| Some(self.add_refresh(sender, None)))
    }

    /// Refresh only specific feeds,
    /// if a running refresh already contains a subset of the requested sources
    /// It will wait for these to complete while also starting new refresh batches for
    /// Feeds that don't have running updates yet.
    pub async fn refresh(&self, sender: &Sender<Action>, source: Vec<Source>) {
        let receivers = self.schedule_refresh(sender, source);
        let handles: Vec<_> = receivers
            .into_iter()
            .map(|mut r| async move {
                let _ = r.wait_for(|v| *v).await;
            })
            .collect();
        futures_util::future::join_all(handles).await;
    }

    /// The non-async variant of schedule_refresh
    pub fn schedule_refresh(
        &self,
        sender: &Sender<Action>,
        source: Vec<Source>,
    ) -> Vec<watch::Receiver<bool>> {
        // figure out what part of the feeds are already scheduled
        let (mut receivers, not_scheduled) = if let Ok(state) = self.state.read() {
            let scheduled_or_not: Vec<Result<RefreshId, Source>> = source
                .iter()
                .map(|requested| {
                    if let Some(refresh_id) = state.currently_running.iter().find_map(|(k, v)| {
                        if v.feeds.contains(requested) {
                            Some(k)
                        } else {
                            None
                        }
                    }) {
                        Ok(*refresh_id)
                    } else {
                        Err(requested.clone())
                    }
                })
                .collect();
            let already_scheduled: Vec<_> = scheduled_or_not
                .iter()
                .filter_map(|r| r.clone().ok())
                .collect();
            let not_scheduled: Vec<_> = scheduled_or_not
                .into_iter()
                .filter_map(|r| r.err())
                .collect();

            let receivers: Vec<_> = already_scheduled
                .into_iter()
                .map(|id| state.currently_running.get(&id).unwrap().receiver.clone())
                .collect();
            (receivers, not_scheduled)
        } else {
            error!("Couldn't lock feed_manager state to schedule_refresh");
            return Vec::new();
        };
        if !not_scheduled.is_empty() {
            receivers.push(self.add_refresh(sender, Some(not_scheduled)));
        }
        receivers
    }

    fn add_refresh(
        &self,
        sender: &Sender<Action>,
        source: Option<Vec<Source>>,
    ) -> watch::Receiver<bool> {
        let (watch_sender, watch_receiver) = watch::channel(false);
        let (sources, is_all) = source
            .map(|s| (s, false))
            .unwrap_or(dbqueries::get_sources().map(|s| (s, true)).unwrap());

        let id = if let Ok(mut state) = self.state.write() {
            let id = state.next_id;
            state.next_id = id + 1;
            state.currently_running.insert(
                id,
                RefreshBatch {
                    represents_full_refresh: is_all,
                    feeds: sources.clone(),
                    receiver: watch_receiver.clone(),
                },
            );
            Some(id)
        } else {
            None
        };

        let sender = sender.clone();
        if let Some(id) = id {
            crate::RUNTIME.spawn(async move {
                send!(sender, Action::StartUpdating);
                if let Err(err) = pipeline(sources.into_iter()).await {
                    error!("Failed to fetch feed: {err}");
                }
                if let Err(e) = watch_sender.send(true) {
                    error!("Failed to send feed done: {e}");
                }
                send!(sender, Action::FeedRefreshed(id));
            });
        }

        watch_receiver
    }

    /// Call this from app.rs when an update is done
    pub(crate) fn refresh_done(sender: Sender<Action>, id: RefreshId) {
        crate::MAINCONTEXT.spawn_local_with_priority(Priority::LOW, async move {
            let all_done = if let Ok(mut state) = FEED_MANAGER.state.write() {
                if state.currently_running.remove(&id).is_none() {
                    error!("Failed to remove refreshId: {id}");
                }
                state.currently_running.is_empty()
            } else {
                error!("refresh_done: Failed to lock feed_manager state");
                false
            };
            if all_done {
                send!(sender, Action::StopUpdating);
                send!(sender, Action::RefreshAllViews);
            }
        });
    }
}

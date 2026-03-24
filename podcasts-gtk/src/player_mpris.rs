// player_mpris.rs
//
// Copyright 2026 nee <nee-git@patchouli.garden>
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

use adw::prelude::*;
use adw::subclass::prelude::*;
use async_channel::Sender;
use gettextrs::gettext;
use glib::clone;
use gst::ClockTime;
use gtk::glib;
use mpris_server::{self, Metadata, PlaybackStatus};
use std::cell::RefCell;
use std::rc::Rc;
use url::Url;

use crate::app::Action;
use crate::config::APP_ID;
use crate::player::{Duration, Player, PlayerExt, PlayerUi, Position, SeekDirection};
use podcasts_data::{Episode, ShowCoverModel};

// Mpris is the protocol that integrates media players
// into Desktop / Mobile Shell user interfaces

#[derive(Debug)]
pub struct PlayerMprisPriv {
    mpris: RefCell<Option<Rc<mpris_server::Player>>>,
    metadata: RefCell<Metadata>,
}

impl Default for PlayerMprisPriv {
    fn default() -> Self {
        let mpris = crate::RUNTIME.block_on(async move {
            let mpris_result = mpris_server::Player::builder(APP_ID)
                .identity(gettext("Podcasts"))
                .desktop_entry(APP_ID)
                .can_raise(true)
                .can_pause(false)
                .can_play(false)
                .can_seek(false)
                .can_set_fullscreen(false)
                .can_go_next(false)
                .can_go_previous(false)
                .build()
                .await;
            match mpris_result {
                Err(e) => {
                    error!("mpris initialization: {e}");
                    None
                }
                Ok(mpris) => Some(Rc::new(mpris)),
            }
        });

        if let Some(mpris) = mpris.as_ref() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                glib::source::Priority::LOW,
                clone!(
                    #[weak]
                    mpris,
                    async move {
                        let task = mpris.run();
                        task.await;
                    }
                ),
            );
        }
        PlayerMprisPriv {
            mpris: RefCell::new(mpris),
            metadata: RefCell::new(Metadata::default()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for PlayerMprisPriv {
    const NAME: &'static str = "PdPlayerMpris";
    type ParentType = glib::Object;
    type Type = PlayerMpris;
}
impl ObjectImpl for PlayerMprisPriv {}
glib::wrapper! {
    pub struct PlayerMpris(ObjectSubclass<PlayerMprisPriv>);
}

impl Default for PlayerMpris {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl PlayerMpris {
    pub(crate) fn init(&self, player: &Player, sender: &Sender<Action>) {
        self.connect_mpris_buttons(player, sender);
        player.bind_ui(self);
        player.connect_local(
            "episode-changed",
            false,
            clone!(
                #[weak]
                player,
                #[weak(rename_to = this)]
                self,
                #[upgrade_or_default]
                move |_| {
                    if let Some(ep) = player.episode().as_ref()
                        && let Some(show) = player.show().as_ref()
                    {
                        this.set_episode(ep, show);
                    }
                    None
                }
            ),
        );
    }

    /// These happen when buttons in the OS's mpris widget are pressed
    fn connect_mpris_buttons(&self, player: &Player, sender: &Sender<Action>) {
        if let Some(mpris) = self.imp().mpris.borrow().as_ref() {
            mpris.connect_play_pause(clone!(
                #[weak]
                player,
                move |mpris| {
                    match mpris.playback_status() {
                        PlaybackStatus::Paused => player.play(),
                        PlaybackStatus::Stopped => player.play(),
                        _ => player.pause(),
                    };
                }
            ));
            mpris.connect_play(clone!(
                #[weak]
                player,
                move |_| player.play()
            ));

            mpris.connect_pause(clone!(
                #[weak]
                player,
                move |_| player.pause()
            ));

            mpris.connect_seek(clone!(
                #[weak]
                player,
                move |_, offset: mpris_server::Time| {
                    let direction = if offset.is_positive() {
                        SeekDirection::Forward
                    } else {
                        SeekDirection::Backwards
                    };
                    player.seek(
                        ClockTime::from_useconds(offset.as_micros().unsigned_abs()),
                        direction,
                    );
                }
            ));

            mpris.connect_raise(clone!(
                #[strong]
                sender,
                move |_| {
                    send_blocking!(sender, Action::RaiseWindow);
                }
            ));
        };
    }

    fn set_episode(&self, episode: &Episode, podcast: &ShowCoverModel) {
        let mut metadata = Metadata::new();
        Self::set_cover(&mut metadata, podcast);
        metadata.set_artist(Some(vec![podcast.title().to_string()]));
        metadata.set_title(Some(episode.title().to_string()));
        metadata.set_length(
            episode
                .duration()
                .map(|s| mpris_server::Time::from_secs(s as i64)),
        );
        self.imp().metadata.replace(metadata.clone());

        self.with_async_context(move |mpris| async move {
            if let Err(err) = mpris.set_metadata(metadata).await {
                warn!("Failed to set MPRIS metadata: {err:?}");
            }
            if let Err(err) = mpris.set_can_pause(true).await {
                warn!("Failed to set MPRIS pause capability: {err:?}");
            }
            if let Err(err) = mpris.set_can_play(true).await {
                warn!("Failed to set MPRIS play capability: {err:?}");
            }
            if let Err(err) = mpris.set_can_seek(true).await {
                warn!("Failed to set MPRIS seek capability: {err:?}");
            }
        });
    }

    fn set_cover(metadata: &mut Metadata, show: &ShowCoverModel) {
        let art_path = crate::download_covers::determin_cover_path(show, None);
        if art_path.exists() {
            metadata.set_art_url(Url::from_file_path(art_path).ok());
        } else {
            // Fallback to web url, it could still work,
            // because of different http agent or no disk space.
            metadata.set_art_url(show.image_uri());
        }
    }

    fn with_metadata<F>(&self, cb: F)
    where
        F: FnOnce(&mut Metadata),
    {
        if let Some(mpris) = self.imp().mpris.borrow().as_ref() {
            let metadata = {
                let mut metadata = self.imp().metadata.borrow_mut();
                cb(&mut metadata);
                metadata.clone()
            };
            self.imp().metadata.replace(metadata);

            crate::MAINCONTEXT.spawn_local_with_priority(
                glib::source::Priority::LOW,
                clone!(
                    #[weak(rename_to = this)]
                    self,
                    #[weak]
                    mpris,
                    async move {
                        let metadata = this.imp().metadata.borrow().clone();
                        if let Err(err) = mpris.set_metadata(metadata).await {
                            error!("failed to update mpris metadata {err}");
                        }
                    }
                ),
            );
        } else {
            info!("no mpris context for setting metadata");
        }
    }

    fn with_async_context<F, Fut>(&self, cb: F)
    where
        F: FnOnce(Rc<mpris_server::Player>) -> Fut + 'static,
        Fut: Future<Output = ()>,
    {
        if let Some(mpris) = self.imp().mpris.borrow().as_ref() {
            crate::MAINCONTEXT.spawn_local_with_priority(
                glib::source::Priority::LOW,
                clone!(
                    #[weak]
                    mpris,
                    async move {
                        cb(mpris).await;
                    }
                ),
            );
        } else {
            info!("no mpris context for async update");
        }
    }
}

impl PlayerUi for PlayerMpris {
    // both handled in custom episode/show callback, so we only have 1 metadata push.
    fn show_changed(&self, _show: &ShowCoverModel) {}
    fn episode_changed(&self, _ep: &Episode) {}

    fn status_changed(&self, status: PlaybackStatus) {
        let status = match status {
            PlaybackStatus::Stopped => PlaybackStatus::Paused,
            status => status,
        };
        self.with_async_context(move |mpris| async move {
            if let Err(err) = mpris.set_playback_status(status).await {
                warn!("Failed to set MPRIS playback status: {err:?}");
            }
        });
    }

    fn show_cover_changed(&self, show: &ShowCoverModel) {
        // avoid pushing too many updates with the same data.
        // episode-changed already sets the cover.
        // Only update when the cover was reset / just downloaded.
        if self.imp().metadata.borrow().art_url().is_none() {
            self.with_metadata(move |metadata| {
                Self::set_cover(metadata, show);
            });
        }
    }

    fn show_cover_reset(&self) {
        self.with_metadata(move |metadata| {
            metadata.set_art_url(None::<Url>);
        })
    }

    fn position_changed(&self, position: Position) {
        let time = mpris_server::Time::from_secs(position.seconds() as i64);
        if let Some(mpris) = self.imp().mpris.borrow().as_ref() {
            mpris.set_position(time);
        }
    }
    fn duration_changed(&self, duration: Duration) {
        self.with_metadata(move |metadata| {
            metadata.set_length(Some(mpris_server::Time::from_secs(
                duration.seconds() as i64
            )));
        });
    }
}

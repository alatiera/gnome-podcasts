// download_progress.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
// Copyright 2023-2024 nee <nee-git@patchouli.garden>
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

use adw::prelude::BinExt;
use adw::subclass::prelude::*;
use glib::clone;
use glib::ParamSpec;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;

use anyhow::{anyhow, bail, Result};

use podcasts_data::dbqueries;
use podcasts_data::downloader::DownloadProgress;

use crate::i18n::i18n;
use crate::manager;
use crate::manager::ActiveProgress;

use once_cell::sync::OnceCell;
use std::cell::Cell;
use std::sync::{Arc, Mutex, TryLockError};
use std::time::Duration;

#[derive(Debug, Default, Properties)]
#[properties(wrapper_type = DownloadProgressBar)]
pub struct DownloadProgressPriv {
    progressbar: gtk::ProgressBar,
    id: OnceCell<i32>,    // episode ID
    listener: Cell<bool>, // lock for update callback
    #[property(get, set)]
    local_size: Cell<u64>,
    #[property(get, set)]
    total_size: Cell<u64>,
}

#[glib::object_subclass]
impl ObjectSubclass for DownloadProgressPriv {
    const NAME: &'static str = "PdDownloadProgress";
    type Type = super::DownloadProgressBar;
    type ParentType = adw::Bin;
}

#[glib::derived_properties]
impl ObjectImpl for DownloadProgressPriv {
    fn constructed(&self) {
        self.parent_constructed();
        self.progressbar.set_visible(false);
        self.progressbar.set_hexpand(true);
        self.progressbar.set_pulse_step(0.0);
        self.progressbar
            .update_property(&[gtk::accessible::Property::Label(&i18n("Download progress"))]);
    }
}
impl WidgetImpl for DownloadProgressPriv {}
impl BinImpl for DownloadProgressPriv {}

impl DownloadProgressPriv {}

glib::wrapper! {
    pub struct DownloadProgressBar(ObjectSubclass<DownloadProgressPriv>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DownloadProgressBar {
    pub fn init(&self, episode_id: i32) {
        let _ = self.imp().id.set(episode_id);
        self.set_child(Some(&self.imp().progressbar));
    }

    /// Notifies when downloading started/stopped
    pub fn connect_state_change<F: Fn(&gtk::ProgressBar) + 'static>(&self, f: F) {
        self.imp().progressbar.connect_visible_notify(f);
    }

    pub fn connect_total_size_change<F>(&self, f: F)
    where
        F: Fn(&DownloadProgressBar, &ParamSpec) + 'static,
    {
        self.connect_notify_local(Some("total_size"), f);
    }

    pub fn id(&self) -> i32 {
        *self.imp().id.get().unwrap()
    }

    fn has_listener(&self) -> bool {
        self.imp().listener.get()
    }

    pub fn check_if_downloading(&self) -> Result<bool> {
        let id = self.id();
        // Check if the episode is being downloaded
        if let Some(prog) = self.active_dl()? {
            // avoid putting up multiple callbacks
            if self.has_listener() {
                return Ok(true);
            }
            debug!("Download is happening, starting download bar.");
            // set a callback that will update the state when the download finishes
            let callback = clone!(@weak self as this => @default-return glib::ControlFlow::Break, move || {
                if let Ok(guard) = manager::ACTIVE_DOWNLOADS.read() {
                    if !guard.contains_key(&id) {
                        this.imp().progressbar.set_visible(false);
                        this.imp().progressbar.set_fraction(0.0);
                        this.imp().listener.set(false);
                        debug!("Download bar done, hiding it now.");
                        return glib::ControlFlow::Break
                    }
                }
                glib::ControlFlow::Continue
            });
            glib::timeout_add_local(Duration::from_millis(250), callback);
            self.imp().listener.set(true);
            self.imp().progressbar.set_visible(true);

            // Setup a callback that will update the total_size label
            // with the http ContentLength header number rather than
            // relying to the RSS feed.
            update_total_size_callback(self, &prog);

            // Setup a callback that will update the progress bar.
            update_progressbar_callback(self, &prog, id);

            return Ok(true);
        }
        Ok(false)
    }

    fn active_dl(&self) -> Result<Option<ActiveProgress>> {
        let id = self.id();
        let m = manager::ACTIVE_DOWNLOADS
            .read()
            .map_err(|_| anyhow!("Failed to get a lock on the mutex."))?;

        return Ok(m.get(&id).cloned());
    }

    pub fn cancel(&self) -> Result<()> {
        let id = self.id();
        if let Some(prog) = self.active_dl()? {
            if let Ok(mut m) = prog.lock() {
                m.cancel();
            }

            // Cancel is not instant so we have to wait a bit
            glib::timeout_add_local(
                Duration::from_millis(50),
                clone!(@weak self as this => @default-return glib::ControlFlow::Break, move || {
                    if let Ok(thing) = this.active_dl() {
                        if thing.is_none() {
                            // Recalculate the widget state
                            if let Err(err) = dbqueries::get_episode_widget_from_id(id) {
                                error!("Error: {}", err);
                            }
                            this.imp().progressbar.set_visible(false);
                            this.imp().progressbar.set_fraction(0.0);
                            return glib::ControlFlow::Break
                        }
                    }

                    glib::ControlFlow::Continue
                }),
            );
        }
        Ok(())
    }

    fn update_progress(&self, local_size: u64, fraction: f64) {
        self.set_local_size(local_size);
        self.imp().progressbar.set_fraction(fraction);
    }
}

// Setup a callback that will update the progress bar.
#[inline]
fn update_progressbar_callback(
    widget: &DownloadProgressBar,
    prog: &Arc<Mutex<manager::Progress>>,
    episode_id: i32,
) {
    let callback = clone!(@weak widget, @strong prog => @default-return glib::ControlFlow::Break, move || {
        progress_bar_helper(&widget, &prog, episode_id)
            .unwrap_or(glib::ControlFlow::Break)
    });
    glib::timeout_add_local(Duration::from_millis(100), callback);
}

fn progress_bar_helper(
    widget: &DownloadProgressBar,
    prog: &Arc<Mutex<manager::Progress>>,
    episode_id: i32,
) -> Result<glib::ControlFlow> {
    let (fraction, downloaded, cancel) = match prog.try_lock() {
        Ok(guard) => (
            guard.get_fraction(),
            guard.get_downloaded(),
            guard.should_cancel(),
        ),
        Err(TryLockError::WouldBlock) => return Ok(glib::ControlFlow::Continue),
        Err(TryLockError::Poisoned(_)) => bail!("Progress Mutex is poisoned"),
    };

    // Update the progress_bar.
    if (0.0..=1.0).contains(&fraction) && (!fraction.is_nan()) {
        widget.update_progress(downloaded, fraction);
    }

    // Check if the download is still active
    let active = match manager::ACTIVE_DOWNLOADS.read() {
        Ok(guard) => guard.contains_key(&episode_id),
        Err(_) => return Err(anyhow!("Failed to get a lock on the mutex.")),
    };

    if (fraction >= 1.0 && !fraction.is_nan()) || !active || cancel {
        Ok(glib::ControlFlow::Break)
    } else {
        Ok(glib::ControlFlow::Continue)
    }
}

// Setup a callback that will update the total_size label
// with the http ContentLength header number rather than
// relying to the RSS feed.
#[inline]
fn update_total_size_callback(widget: &DownloadProgressBar, prog: &Arc<Mutex<manager::Progress>>) {
    let callback = clone!(@strong prog, @weak widget => @default-return glib::ControlFlow::Break, move || {
        total_size_helper(&widget, &prog).unwrap_or(glib::ControlFlow::Continue)
    });
    glib::timeout_add_local(Duration::from_millis(100), callback);
}

fn total_size_helper(
    widget: &DownloadProgressBar,
    prog: &Arc<Mutex<manager::Progress>>,
) -> Result<glib::ControlFlow> {
    // Get the total_bytes.
    let total_bytes = match prog.try_lock() {
        Ok(guard) => guard.get_size(),
        Err(TryLockError::WouldBlock) => return Ok(glib::ControlFlow::Continue),
        Err(TryLockError::Poisoned(_)) => bail!("Progress Mutex is poisoned"),
    };

    debug!("Total Size: {}", total_bytes);
    if total_bytes != 0 {
        // Update the total_size label
        widget.set_total_size(total_bytes);

        // Do not call again the callback
        Ok(glib::ControlFlow::Break)
    } else {
        Ok(glib::ControlFlow::Continue)
    }
}

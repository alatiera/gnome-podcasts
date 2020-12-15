// utils.rs
//
// Copyright 2018 Jordan Petridis <jpetridis@gnome.org>
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

use gdk_pixbuf::Pixbuf;
use gio::ActionMapExt;
use glib::clone;
use glib::Sender;
use glib::Variant;
use glib::{self, object::WeakRef};
use glib::{IsA, Object};
use gtk;
use gtk::prelude::*;
use gtk::Widget;

use anyhow::{anyhow, Result};
use chrono::prelude::*;
use crossbeam_channel::{bounded, unbounded};
use fragile::Fragile;
use rayon;
use regex::Regex;
use reqwest;
use serde_json::Value;
use url::Url;

// use podcasts_data::feed;
use podcasts_data::dbqueries;
use podcasts_data::downloader;
use podcasts_data::errors::DownloadError;
use podcasts_data::opml;
use podcasts_data::pipeline::pipeline;
use podcasts_data::utils::{calculate_hash, checkup};
use podcasts_data::Source;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};

use crate::app::Action;

use crate::i18n::i18n;
use chrono::Duration;

/// Copied from the gtk-macros crate
///
/// Send an event through a glib::Sender
///
/// - Before:
///
///     Example:
///
///     ```no_run
///     sender.send(Action::DoThing).expect("Failed to send DoThing through the glib channel?");
///     ```
///
/// - After:
///
///     Example:
///
///     ```no_run
///     send!(self.sender, Action::DoThing);
///     ```
#[macro_export]
macro_rules! send {
    ($sender:expr, $action:expr) => {
        if let Err(err) = $sender.send($action) {
            panic!(format!(
                "Failed to send \"{}\" action due to {}",
                stringify!($action),
                err
            ));
        }
    };
}

/// Creates an action named `name` in the action map `T with the handler `F`
pub fn make_action<T, F>(thing: &T, name: &str, action: F)
where
    T: ActionMapExt,
    F: Fn(&gio::SimpleAction, Option<&Variant>) + 'static,
{
    // Create a stateless, parameterless action
    let act = gio::SimpleAction::new(name, None);
    // Connect the handler
    act.connect_activate(action);
    // Add it to the map
    thing.add_action(&act);
}

/// Lazy evaluates and loads widgets to the parent `container` widget.
///
/// Accepts an `IntoIterator`, `data`, as the source from which each widget
/// will be constructed. An `FnMut` function that returns the desired
/// widget should be passed as the widget `constructor`. You can also specify
/// a `callback` that will be executed when the iteration finish.
///
/// ```no_run
/// # struct Message;
/// # struct MessageWidget(gtk::Label);
///
/// # impl MessageWidget {
/// #    fn new(_: Message) -> Self {
/// #        MessageWidget(gtk::Label::new("A message"))
/// #    }
/// # }
///
/// let messages: Vec<Message> = Vec::new();
/// let list = gtk::ListBox::new();
/// let constructor = |m| MessageWidget::new(m).0;
/// lazy_load(messages, list, constructor, || {});
/// ```
///
/// If you have already constructed the widgets and only want to
/// load them to the parent you can pass a closure that returns it's
/// own argument to the constructor.
///
/// ```no_run
/// # use std::collections::binary_heap::BinaryHeap;
/// let widgets: BinaryHeap<gtk::Button> = BinaryHeap::new();
/// let list = gtk::ListBox::new();
/// lazy_load(widgets, list, |w| w, || {});
/// ```
pub(crate) fn lazy_load<T, C, F, W, U>(
    data: T,
    container: WeakRef<C>,
    mut constructor: F,
    callback: U,
) where
    T: IntoIterator + 'static,
    T::Item: 'static,
    C: IsA<Object> + ContainerExt + 'static,
    F: FnMut(T::Item) -> W + 'static,
    W: IsA<Widget> + WidgetExt,
    U: Fn() + 'static,
{
    let func = move |x| {
        let container = match container.upgrade() {
            Some(c) => c,
            None => return,
        };

        let widget = constructor(x);
        container.add(&widget);
        widget.show();
    };
    lazy_load_full(data, func, callback);
}

/// Iterate over `data` and execute `func` using a `gtk::idle_add()`,
/// when the iteration finishes, it executes `finish_callback`.
///
/// This is a more flexible version of `lazy_load` with less constrains.
/// If you just want to lazy add `widgets` to a `container` check if
/// `lazy_load` fits your needs first.
#[allow(clippy::redundant_closure)]
pub(crate) fn lazy_load_full<T, F, U>(data: T, mut func: F, finish_callback: U)
where
    T: IntoIterator + 'static,
    T::Item: 'static,
    F: FnMut(T::Item) + 'static,
    U: Fn() + 'static,
{
    let mut data = data.into_iter();
    gtk::idle_add(move || {
        data.next()
            .map(|x| func(x))
            .map(|_| glib::Continue(true))
            .unwrap_or_else(|| {
                finish_callback();
                glib::Continue(false)
            })
    });
}

// Kudos to Julian Sparber
// https://blogs.gnome.org/jsparber/2018/04/29/animate-a-scrolledwindow/
#[allow(clippy::float_cmp)]
pub(crate) fn smooth_scroll_to(view: &gtk::ScrolledWindow, target: &gtk::Adjustment) {
    if let Some(adj) = view.get_vadjustment() {
        if let Some(clock) = view.get_frame_clock() {
            let duration = 200;
            let start = adj.get_value();
            let end = target.get_value();
            let start_time = clock.get_frame_time();
            let end_time = start_time + 1000 * duration;

            view.add_tick_callback(move |_, clock| {
                let now = clock.get_frame_time();
                // FIXME: `adj.get_value != end` is a float comparison...
                if now < end_time && adj.get_value().abs() != end.abs() {
                    let mut t = (now - start_time) as f64 / (end_time - start_time) as f64;
                    t = ease_out_cubic(t);
                    adj.set_value(start + t * (end - start));
                    Continue(true)
                } else {
                    adj.set_value(end);
                    Continue(false)
                }
            });
        }
    }
}

// From clutter-easing.c, based on Robert Penner's
// infamous easing equations, MIT license.
fn ease_out_cubic(t: f64) -> f64 {
    let p = t - 1f64;
    p * p * p + 1f64
}

lazy_static! {
    static ref IGNORESHOWS: Arc<Mutex<HashSet<i32>>> = Arc::new(Mutex::new(HashSet::new()));
}

pub(crate) fn ignore_show(id: i32) -> Result<bool> {
    IGNORESHOWS
        .lock()
        .map(|mut guard| guard.insert(id))
        .map_err(|err| anyhow!("{}", err))
}

pub(crate) fn unignore_show(id: i32) -> Result<bool> {
    IGNORESHOWS
        .lock()
        .map(|mut guard| guard.remove(&id))
        .map_err(|err| anyhow!("{}", err))
}

pub(crate) fn get_ignored_shows() -> Result<Vec<i32>> {
    IGNORESHOWS
        .lock()
        .map(|guard| guard.iter().cloned().collect::<Vec<_>>())
        .map_err(|err| anyhow!("{}", err))
}

pub(crate) fn cleanup(cleanup_date: DateTime<Utc>) {
    checkup(cleanup_date)
        .map_err(|err| error!("Check up failed: {}", err))
        .ok();
}

/// Schedule feed refresh
/// If `source` is None, Refreshes all sources in the database.
/// Current implementation ignores update request if another update is already running
pub(crate) fn schedule_refresh(source: Option<Vec<Source>>, sender: Sender<Action>) {
    // If we try to update the whole db,
    // Exit early if `source` table is empty
    if source.is_none() {
        match dbqueries::is_source_populated(&[]) {
            Ok(false) => {
                info!("No source of feeds where found, returning");
                return;
            }
            Err(err) => debug_assert!(false, err),
            _ => (),
        };
    }

    send!(sender, Action::UpdateFeed(source));
}

/// Update the rss feed(s) originating from `source`.
/// If `source` is None, Fetches all the `Source` entries in the database and updates them.
/// Do not call this function directly unless you are sure no other updates are running.
/// Use `schedule_refresh()` instead
pub(crate) fn refresh_feed(source: Option<Vec<Source>>, sender: Sender<Action>) {
    let (up_sender, up_receiver) = bounded(1);
    send!(sender, Action::ShowUpdateNotif(up_receiver));

    if let Some(s) = source {
        // Refresh only specified feeds
        tokio::spawn(async move {
            pipeline(s).await;
            up_sender
                .send(true)
                .expect("Channel was dropped unexpectedly");
        })
    } else {
        // Refresh all the feeds
        tokio::spawn(async move {
            let sources = dbqueries::get_sources().map(|s| s.into_iter()).unwrap();
            pipeline(sources).await;
            up_sender
                .send(true)
                .expect("Channel was dropped unexpectedly");
        })
    };
}

lazy_static! {
    static ref CACHED_PIXBUFS: RwLock<HashMap<(i32, u32), Mutex<Fragile<Pixbuf>>>> =
        RwLock::new(HashMap::new());
    static ref COVER_DL_REGISTRY: RwLock<HashSet<i32>> = RwLock::new(HashSet::new());
    static ref THREADPOOL: rayon::ThreadPool = rayon::ThreadPoolBuilder::new().build().unwrap();
}

// Determine whether a cached image is valid.
//
// A cached image is valid for a maximum of 4 weeks from the time of its previous download.
// Otherwise, a cached image is only valid so long as the hash of its URI remains unchanged.
fn cached_image_valid(pd: &podcasts_data::ShowCoverModel) -> bool {
    let cache_valid_duration = Duration::weeks(4);
    if Utc::now()
        .naive_utc()
        .signed_duration_since(*pd.image_cached())
        > cache_valid_duration
    {
        return false;
    }
    if let Some(new) = &pd.image_uri() {
        if let Some(orig) = pd.image_uri_hash() {
            return calculate_hash(new) == orig;
        }
    }
    false
}

// Since gdk_pixbuf::Pixbuf is reference counted and every episode,
// use the cover of the Podcast Feed/Show, We can only create a Pixbuf
// cover per show and pass around the Rc pointer.
//
// GObjects do not implement Send trait, so SendCell is a way around that.
// Also lazy_static requires Sync trait, so that's what the mutexes are.
// TODO: maybe use something that would just scale to requested size?
pub(crate) fn set_image_from_path(image: &gtk::Image, show_id: i32, size: u32) -> Result<()> {
    if let Ok(hashmap) = CACHED_PIXBUFS.read() {
        if let Ok(pd) = dbqueries::get_podcast_cover_from_id(show_id) {
            // If the image is still valid, check if the requested (cover + size) is already in the
            // cache and if so do an early return after that.
            if cached_image_valid(&pd) {
                if let Some(guard) = hashmap.get(&(show_id, size)) {
                    guard
                        .lock()
                        .map_err(|err| anyhow!("Fragile Mutex: {}", err))
                        .and_then(|fragile| {
                            fragile
                                .try_get()
                                .map(|px| image.set_from_pixbuf(Some(px)))
                                .map_err(From::from)
                        })?;

                    return Ok(());
                }
            }
        }
    }

    // Check if there's an active download about this show cover.
    // If there is, a callback will be set so this function will be called again.
    // If the download succeeds, there should be a quick return from the pixbuf cache_image
    // If it fails another download will be scheduled.
    if let Ok(guard) = COVER_DL_REGISTRY.read() {
        if guard.contains(&show_id) {
            let callback = clone!(@weak image => @default-return glib::Continue(false), move || {
                 let _ = set_image_from_path(&image, show_id, size);
                 glib::Continue(false)
            });
            gtk::timeout_add(250, callback);
            return Ok(());
        }
    }

    let (sender, receiver) = unbounded();
    if let Ok(mut guard) = COVER_DL_REGISTRY.write() {
        // Add the id to the hashmap from the main thread to avoid queuing more than one downloads.
        guard.insert(show_id);
        drop(guard);

        THREADPOOL.spawn(move || {
            // This operation is polling and will block the thread till the download is finished
            if let Ok(pd) = dbqueries::get_podcast_cover_from_id(show_id) {
                sender
                    .send(downloader::cache_image(&pd))
                    .expect("channel was dropped unexpectedly");
            }

            if let Ok(mut guard) = COVER_DL_REGISTRY.write() {
                guard.remove(&show_id);
            }
        });
    }

    let image = image.clone();
    let s = size as i32;
    gtk::timeout_add(25, move || {
        use crossbeam_channel::TryRecvError;

        match receiver.try_recv() {
            Err(TryRecvError::Empty) => glib::Continue(true),
            Err(TryRecvError::Disconnected) => glib::Continue(false),
            Ok(path) => {
                match path {
                    Ok(path) => {
                        if let Ok(px) = Pixbuf::from_file_at_scale(&path, s, s, true) {
                            if let Ok(mut hashmap) = CACHED_PIXBUFS.write() {
                                hashmap
                                    .insert((show_id, size), Mutex::new(Fragile::new(px.clone())));
                                image.set_from_pixbuf(Some(&px));
                            }
                        }
                    }
                    Err(DownloadError::NoImageLocation) => {
                        image.set_from_icon_name(
                            Some("image-x-generic-symbolic"),
                            gtk::IconSize::__Unknown(s),
                        );
                    }
                    _ => {}
                }
                if let Ok(pd) = dbqueries::get_podcast_from_id(show_id) {
                    if let Err(err) = pd.update_image_cache_values() {
                        error!(
                            "Failed to update the image's cache values for podcast {}: {}",
                            pd.title(),
                            err
                        )
                    }
                }
                glib::Continue(false)
            }
        }
    });
    Ok(())
}

// FIXME: the signature should be `fn foo(s: Url) -> Result<Url>`
pub(crate) async fn itunes_to_rss(url: &str) -> Result<String> {
    let id = itunes_id_from_url(url).ok_or_else(|| anyhow!("Failed to find an iTunes ID."))?;
    itunes_lookup_id(id).await
}

fn itunes_id_from_url(url: &str) -> Option<u32> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"/id([0-9]+)").unwrap();
    }

    // Get the itunes id from the url
    let foo = RE.captures_iter(url).nth(0)?.get(1)?.as_str();
    // Parse it to a u32, this *should* never fail
    foo.parse::<u32>().ok()
}

async fn itunes_lookup_id(id: u32) -> Result<String> {
    let url = format!("https://itunes.apple.com/lookup?id={}&entity=podcast", id);
    let req: Value = reqwest::get(&url).await?.json().await?;
    let rssurl = || -> Option<&str> { req.get("results")?.get(0)?.get("feedUrl")?.as_str() };
    rssurl()
        .map(From::from)
        .ok_or_else(|| anyhow!("Failed to get url from itunes response"))
}

pub(crate) async fn soundcloud_to_rss(url: &Url) -> Result<Url> {
    // Turn: https://soundcloud.com/chapo-trap-house
    // into: https://feeds.soundcloud.com/users/soundcloud:users:211911700/sounds.rss
    let id = soundcloud_lookup_id(url)
        .await
        .ok_or_else(|| anyhow!("Failed to find a soundcloud ID."))?;
    let url = format!(
        "https://feeds.soundcloud.com/users/soundcloud:users:{}/sounds.rss",
        id
    );
    Ok(Url::parse(&url)?)
}

async fn soundcloud_lookup_id(url: &Url) -> Option<u64> {
    // lookup the users: id for a soundcloud url
    lazy_static! {
        static ref RE: Regex = Regex::new(r"soundcloud://users:([0-9]+)").unwrap();
    }
    let url_str = url.to_string();
    let response_text = reqwest::get(&url_str).await.ok()?.text().await.ok()?;
    let id = RE.captures_iter(&response_text).nth(0)?.get(1)?.as_str();
    // Parse it to a u64, this *should* never fail
    id.parse::<u64>().ok()
}

pub(crate) fn on_import_clicked(window: &gtk::ApplicationWindow, sender: &Sender<Action>) {
    use gtk::{FileChooserAction, FileChooserNative, FileFilter, ResponseType};

    // Create the FileChooser Dialog
    let dialog = FileChooserNative::new(
        Some(i18n("Select the file from which to you want to import shows.").as_str()),
        Some(window),
        FileChooserAction::Open,
        Some(i18n("_Import").as_str()),
        None,
    );

    // Do not show hidden(.thing) files
    dialog.set_show_hidden(false);

    // Set a filter to show only xml files
    let filter = FileFilter::new();
    FileFilter::set_name(&filter, Some(i18n("OPML file").as_str()));
    filter.add_mime_type("application/xml");
    filter.add_mime_type("text/xml");
    filter.add_mime_type("text/x-opml");
    dialog.add_filter(&filter);

    let resp = dialog.run();
    debug!("Dialog Response {}", resp);
    if resp == ResponseType::Accept {
        if let Some(filename) = dialog.get_filename() {
            debug!("File selected: {:?}", filename);

            rayon::spawn(clone!(@strong sender => move || {
                // Parse the file and import the feeds
                if let Ok(sources) = opml::import_from_file(filename) {
                    // Refresh the successfully parsed feeds to index them
                    schedule_refresh(Some(sources), sender)
                } else {
                    let text = i18n("Failed to parse the imported file");
                    send!(sender, Action::ErrorNotification(text));
                }
            }))
        } else {
            let text = i18n("Selected file could not be accessed.");
            send!(sender, Action::ErrorNotification(text))
        }
    }
}

pub(crate) fn on_export_clicked(window: &gtk::ApplicationWindow, sender: &Sender<Action>) {
    use gtk::{FileChooserAction, FileChooserNative, FileFilter, ResponseType};

    // Create the FileChooser Dialog
    let dialog = FileChooserNative::new(
        Some(i18n("Export shows to…").as_str()),
        Some(window),
        FileChooserAction::Save,
        Some(i18n("_Export").as_str()),
        Some(i18n("_Cancel").as_str()),
    );

    // Translators: This is the string of the suggested name for the exported opml file
    dialog.set_current_name(format!("{}.opml", i18n("gnome-podcasts-exported-shows")));

    // Do not show hidden(.thing) files
    dialog.set_show_hidden(false);

    // Set a filter to show only xml files
    let filter = FileFilter::new();
    FileFilter::set_name(&filter, Some(i18n("OPML file").as_str()));
    filter.add_mime_type("application/xml");
    filter.add_mime_type("text/xml");
    filter.add_mime_type("text/x-opml");
    dialog.add_filter(&filter);

    let resp = dialog.run();
    debug!("Dialog Response {}", resp);
    if resp == ResponseType::Accept {
        if let Some(filename) = dialog.get_filename() {
            debug!("File selected: {:?}", filename);

            rayon::spawn(clone!(@strong sender => move || {
                if opml::export_from_db(filename, i18n("GNOME Podcasts Subscriptions").as_str()).is_err() {
                    let text = i18n("Failed to export podcasts");
                    send!(sender, Action::ErrorNotification(text));
                }
            }))
        } else {
            let text = i18n("Selected file could not be accessed.");
            send!(sender, Action::ErrorNotification(text));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    // use podcasts_data::Source;
    // use podcasts_data::dbqueries;

    // #[test]
    // This test inserts an rss feed to your `XDG_DATA/podcasts/podcasts.db` so we make it explicit
    // to run it.
    // #[ignore]
    // Disabled till https://gitlab.gnome.org/World/podcasts/issues/56
    // fn test_set_image_from_path() {
    //     let url = "https://web.archive.org/web/20180120110727if_/https://rss.acast.com/thetipoff";
    // Create and index a source
    //     let source = Source::from_url(url).unwrap();
    // Copy it's id
    //     let sid = source.id();
    //     pipeline::run(vec![source], true).unwrap();

    // Get the Podcast
    //     let img = gtk::Image::new();
    //     let pd = dbqueries::get_podcast_from_source_id(sid).unwrap().into();
    //     let pxbuf = set_image_from_path(&img, Arc::new(pd), 256);
    //     assert!(pxbuf.is_ok());
    // }

    #[tokio::test]
    async fn test_itunes_to_rss() -> Result<()> {
        let itunes_url = "https://itunes.apple.com/podcast/id1195206601";
        let rss_url = String::from("https://rss.acast.com/intercepted-with-jeremy-scahill");
        assert_eq!(rss_url, itunes_to_rss(itunes_url).await?);

        let itunes_url = "https://itunes.apple.com/podcast/id000000000000000";
        assert!(itunes_to_rss(itunes_url).await.is_err());
        Ok(())
    }

    #[test]
    fn test_itunes_id() -> Result<()> {
        let id = 1195206601;
        let itunes_url = "https://itunes.apple.com/podcast/id1195206601";
        assert_eq!(id, itunes_id_from_url(itunes_url).unwrap());
        Ok(())
    }

    #[tokio::test]
    async fn test_itunes_lookup_id() -> Result<()> {
        let id = 1195206601;
        let rss_url = "https://rss.acast.com/intercepted-with-jeremy-scahill";
        assert_eq!(rss_url, itunes_lookup_id(id).await?);

        let id = 000000000;
        assert!(itunes_lookup_id(id).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_soundcloud_to_rss() -> Result<()> {
        let soundcloud_url = Url::parse("https://soundcloud.com/chapo-trap-house")?;
        let rss_url = String::from(
            "https://feeds.soundcloud.com/users/soundcloud:users:211911700/sounds.rss",
        );
        assert_eq!(
            Url::parse(&rss_url)?,
            soundcloud_to_rss(&soundcloud_url).await?
        );

        let soundcloud_url =
            Url::parse("https://soundcloud.com/id000000000000000ajlsfhlsfhwoerzuweioh")?;
        assert!(soundcloud_to_rss(&soundcloud_url).await.is_err());
        Ok(())
    }
}

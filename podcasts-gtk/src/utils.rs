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

use anyhow::{Context, Result, anyhow, bail};
use async_channel::Sender;
use async_channel::unbounded;
use chrono::prelude::*;
use formatx::formatx;
use futures_util::StreamExt;
use gettextrs::{gettext, ngettext};
use glib::object::WeakRef;
use gtk::FileFilter;
use gtk::Widget;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use url::Url;

use crate::app::Action;
use podcasts_data::dbqueries;
use podcasts_data::downloader::client_builder;
use podcasts_data::feed_manager::FEED_MANAGER;
use podcasts_data::opml;
use podcasts_data::utils::checkup;
use podcasts_data::{ShowId, Source};

/// Copied from the gtk-macros crate
///
/// Send an event through a async_channel::Sender
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
        if let Err(err) = $sender.send($action).await {
            panic!(
                "Failed to send \"{}\" action due to {}",
                stringify!($action),
                err
            );
        }
    };
}

/// Same as send! but not async.
/// Should not be used from async functions.
#[macro_export]
macro_rules! send_blocking {
    ($sender:expr, $action:expr) => {
        if let Err(err) = $sender.send_blocking($action) {
            panic!(
                "Failed to send \"{}\" action due to {}",
                stringify!($action),
                err
            );
        }
    };
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
pub(crate) async fn lazy_load<C, W, T>(
    data: Vec<T>,
    container: WeakRef<W>,
    constructor: C,
) -> Vec<Result<(), glib::JoinError>>
where
    T: 'static,
    W: IsA<Widget> + Sized + 'static,
    C: Fn(T) -> W + 'static,
{
    let (sender, receiver) = unbounded::<W>();

    let h1 =
        crate::MAINCONTEXT.spawn_local_with_priority(glib::source::Priority::LOW, async move {
            let mut total_duration = Duration::default();
            let mut count = 0;

            for item in data {
                let start = Instant::now();
                let widget = constructor(item);

                let duration = start.elapsed();
                trace!("Created single widget in: {:?}", duration);
                total_duration += duration;
                count += 1;

                if let Err(err) = sender.send(widget).await {
                    debug!("Got SendError, Channel is closed: {}", err);
                    return;
                };

                tokio::task::yield_now().await;
            }

            debug!("Created {} widgets in: {:?}", count, total_duration);
        });

    let h2 = crate::MAINCONTEXT.spawn_local_with_priority(
        glib::source::Priority::DEFAULT_IDLE,
        async move {
            receiver
                .chunks(25)
                .for_each(move |widgets| {
                    trace!("Received {} widgets", &widgets.len());
                    insert_widgets_idle(widgets, container.clone())
                })
                .await
        },
    );

    futures_util::future::join_all([h1, h2]).await
}

async fn insert_widgets_idle<W>(data: Vec<W>, container: WeakRef<W>)
where
    W: IsA<Widget> + Sized + 'static,
{
    let widget_count = data.len();

    let mut count = 0;
    let mut start = Instant::now();

    let mut batch_construction_time_total = Duration::default();

    for widget in data {
        let w_start = Instant::now();
        insert_widget_dynamic(widget, &container);
        let w_duration = w_start.elapsed();
        trace!("Inserted single widget in: {:?}", w_duration);
        count += 1;
        batch_construction_time_total += w_duration;

        let duration = start.elapsed();
        if duration > Duration::from_millis(1) {
            trace!("Inserted batch of {} widgets in: {:?}", count, duration);
            tokio::task::yield_now().await;
            count = 0;
            start = Instant::now();
        }
    }

    debug!(
        "Inserted {} widgets in: {:?}",
        widget_count, batch_construction_time_total
    );
}

fn insert_widget_dynamic<W: IsA<Widget> + Sized>(widget: W, container: &WeakRef<W>) {
    let container = match container.upgrade() {
        Some(c) => c,
        None => return,
    };

    if let Some(listbox) = container.dynamic_cast_ref::<gtk::ListBox>() {
        listbox.append(&widget);
    } else if let Some(flowbox) = container.dynamic_cast_ref::<gtk::FlowBox>() {
        flowbox.append(&widget);
    } else {
        unreachable!("Failed to downcast widget. {}", container.value_type());
    }

    widget.set_visible(true);
}

static IGNORESHOWS: LazyLock<Arc<Mutex<HashSet<ShowId>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(HashSet::new())));

pub(crate) fn ignore_show(id: ShowId) -> Result<bool> {
    IGNORESHOWS
        .lock()
        .map(|mut guard| guard.insert(id))
        .map_err(|err| anyhow!("{err}"))
}

pub(crate) fn unignore_show(id: ShowId) -> Result<bool> {
    IGNORESHOWS
        .lock()
        .map(|mut guard| guard.remove(&id))
        .map_err(|err| anyhow!("{err}"))
}

pub(crate) fn get_ignored_shows() -> Result<Vec<ShowId>> {
    IGNORESHOWS
        .lock()
        .map(|guard| guard.iter().cloned().collect::<Vec<_>>())
        .map_err(|err| anyhow!("{err}"))
}

pub(crate) fn cleanup(cleanup_date: DateTime<Utc>) {
    if let Err(err) = checkup(cleanup_date) {
        error!("Check up failed: {err}");
    }
}

pub(crate) async fn subscribe(sender: &Sender<Action>, feed: String) {
    let mut error_source = None; // <- auto unsub from this
    if let Err(e) = async {
        let source = dbqueries::get_source_from_uri(&feed).or_else(|_| Source::from_url(&feed))?;
        error_source = Some(source.clone());
        let source_id = source.id();
        info!("Subscribing to {feed}");
        let _ = FEED_MANAGER.refresh(vec![source]).await;
        let show = dbqueries::get_podcast_from_source_id(source_id)?;
        if let Err(e) = podcasts_data::sync::Show::store_by_uri(
            feed.to_string(),
            podcasts_data::sync::ShowAction::Added,
        ) {
            error!("Failed store subscription for sync {e}");
        }
        send!(sender, Action::QuickSyncNextcloud);
        send!(sender, Action::RefreshAllViews);
        send!(sender, Action::GoToShow(Arc::new(show.clone())));
        Ok::<(), anyhow::Error>(())
    }
    .await
    {
        error!("Failed to subscribe: {feed} {e}");
        // auto unsubscribe
        if let Some(error_source) = error_source {
            // only unsub if no Show was imported from the source.
            if dbqueries::get_podcast_from_source_id(error_source.id()).is_err() {
                if let Err(remove_err) = dbqueries::remove_source(&error_source) {
                    error!("failed to remove failed source! {remove_err} {feed}");
                } else {
                    info!("auto removed source that failed to import {feed}");
                }
            }
        }
        // TODO show the actual error (like "content didn't start with rss feed"),
        // but pipeline doesn't pass useful errors yet
        send!(
            sender,
            Action::ErrorNotification(
                formatx!(gettext("Failed to subscribe to feed: {}"), feed)
                    .expect("Could not format translatable string")
            )
        );
    }
}

// FIXME: the signature should be `fn foo(s: Url) -> Result<Url>`
pub(crate) async fn itunes_to_rss(url: &str) -> Result<String> {
    let id = itunes_id_from_url(url).ok_or_else(|| anyhow!("Failed to find an iTunes ID."))?;
    itunes_lookup_id(id).await
}

fn itunes_id_from_url(url: &str) -> Option<u32> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/id([0-9]+)").unwrap());

    // Get the itunes id from the url
    let itunes_id = RE.captures_iter(url).next()?.get(1)?.as_str();
    // Parse it to a u32, this *should* never fail
    itunes_id.parse::<u32>().ok()
}

async fn itunes_lookup_id(id: u32) -> Result<String> {
    let url = format!("https://itunes.apple.com/lookup?id={}&entity=podcast", id);
    let req: Value = client_builder()
        .build()?
        .get(&url)
        .send()
        .await?
        .json()
        .await?;
    let rssurl = || -> Option<&str> { req.get("results")?.get(0)?.get("feedUrl")?.as_str() };
    rssurl()
        .map(From::from)
        .ok_or_else(|| anyhow!("Failed to get url from itunes response"))
}

/// Convert soundcloud page links to rss feed links.
/// Works for users and playlists.
pub(crate) async fn soundcloud_to_rss(url: &Url) -> Result<Url> {
    // Turn: https://soundcloud.com/chapo-trap-house
    // into: https://feeds.soundcloud.com/users/soundcloud:users:211911700/sounds.rss
    let (user_id, playlist_id) = soundcloud_lookup_id(url)
        .await
        .ok_or_else(|| anyhow!("Failed to find a soundcloud ID."))?;
    if playlist_id != 0 {
        let url = format!(
            "https://feeds.soundcloud.com/playlists/soundcloud:playlists:{}/sounds.rss",
            playlist_id
        );
        Ok(Url::parse(&url)?)
    } else if user_id != 0 {
        let url = format!(
            "https://feeds.soundcloud.com/users/soundcloud:users:{}/sounds.rss",
            user_id
        );
        Ok(Url::parse(&url)?)
    } else {
        Err(anyhow!("No valid id's in soundcloud page."))
    }
}

/// Extract (user, playlist) id's from a soundcloud page.
/// The id's are 0 if none was found.
/// If fetching the html page fails an Error is returned.
async fn soundcloud_lookup_id(url: &Url) -> Option<(u64, u64)> {
    static RE_U: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"soundcloud://users:([0-9]+)").unwrap());
    static RE_P: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"soundcloud://playlists:([0-9]+)").unwrap());

    let url_str = url.to_string();
    let client = client_builder().build().ok()?;
    let response = client.get(&url_str).send();
    let response_text = response.await.ok()?.text().await.ok()?;
    let user_id = RE_U
        .captures_iter(&response_text)
        .next()
        .and_then(|r| r.get(1).map(|u| u.as_str()));
    let playlist_id = RE_P
        .captures_iter(&response_text)
        .next()
        .and_then(|r| r.get(1).map(|u| u.as_str()));
    // Parse it to a u64, this *should* never fail
    Some((
        user_id.and_then(|id| id.parse::<u64>().ok()).unwrap_or(0),
        playlist_id
            .and_then(|id| id.parse::<u64>().ok())
            .unwrap_or(0),
    ))
}

pub(crate) async fn on_import_clicked(window: &gtk::ApplicationWindow, sender: &Sender<Action>) {
    // Set a filter to show only xml files
    let filter = FileFilter::new();
    FileFilter::set_name(&filter, Some(gettext("OPML file").as_str()));
    filter.add_mime_type("application/xml");
    filter.add_mime_type("text/xml");
    filter.add_mime_type("text/x-opml");

    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);

    // Create the FileChooser Dialog
    let dialog = gtk::FileDialog::builder()
        .title(gettext(
            "Select the file from which to you want to import shows.",
        ))
        .accept_label(gettext("_Import"))
        .filters(&filters)
        .build();

    if let Ok(file) = dialog.open_future(Some(window)).await {
        if let Some(path) = file.peek_path() {
            // spawn a thread to avoid blocking ui during import
            let result = gio::spawn_blocking(move || {
                // Parse the file and import the feeds
                opml::import_from_file(path)
                    .map(|sources| {
                        let sync_urls: Vec<_> =
                            sources.iter().map(|s| s.uri().to_owned()).collect();
                        if let Err(e) =
                            podcasts_data::sync::Show::store_multiple_subscriptions(&sync_urls[..])
                        {
                            error!("Failed store subscriptions for sync from import {e}");
                        }

                        // Refresh the successfully parsed feeds to index them
                        FEED_MANAGER.schedule_refresh(sources);
                    })
                    .context("PARSE")
            })
            .await;

            send!(sender, Action::QuickSyncNextcloud);

            if let Err(err) = result.unwrap_or_else(|e| bail!("Import Thread Error {e:#?}")) {
                let text = formatx!(gettext("Failed to parse the imported file {}",), err,)
                    .expect("Could not format translatable string");
                send!(sender, Action::ErrorNotification(text));
            }
        }
    };
}

pub(crate) async fn on_export_clicked(window: &gtk::ApplicationWindow, sender: &Sender<Action>) {
    // Set a filter to show only xml files
    let filter = FileFilter::new();
    FileFilter::set_name(&filter, Some(&gettext("OPML file")));
    filter.add_mime_type("application/xml");
    filter.add_mime_type("text/xml");
    filter.add_mime_type("text/x-opml");

    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);

    // Create the FileChooser Dialog
    let dialog = gtk::FileDialog::builder()
        // Translators: Show as a noun, meaning Podcast-Shows.
        .title(gettext("Export shows to…"))
        .accept_label(gettext("_Export"))
        .initial_name(format!(
            "{}.opml",
            // Translators: This is the string of the suggested name for the exported opml file
            gettext("gnome-podcasts-exported-shows")
        ))
        .filters(&filters)
        .build();

    if let Ok(file) = dialog.save_future(Some(window)).await {
        if let Some(path) = file.peek_path() {
            debug!("File selected: {:?}", path);
            let result = gio::spawn_blocking(move || {
                opml::export_from_db(path, &gettext("GNOME Podcasts Subscriptions"))
            })
            .await;
            if let Ok(Err(err)) = result {
                let text = gettext("Failed to export podcasts");
                error!("Failed to export podcasts: {err}");
                send!(sender, Action::ErrorNotification(text));
            }
        }
    };
}

/// Only works for Durations that are positive.
/// Call now.signed_duration_since(date_in_the_past) to get the duration.
pub(crate) fn relative_time(duration: chrono::Duration) -> String {
    if duration.num_seconds() < 60 {
        gettext("Just now")
    } else if duration.num_minutes() < 60 {
        let time = duration.num_minutes();
        formatx!(
            ngettext("{} minute ago", "{} minutes ago", time as u32),
            time,
        )
        .expect("Could not format translatable string")
    } else if duration.num_hours() < 24 {
        let time = duration.num_hours();
        formatx!(ngettext("{} hour ago", "{} hours ago", time as u32), time)
            .expect("Could not format translatable string")
    } else if duration.num_days() < 31 {
        let time = duration.num_days();
        formatx!(ngettext("{} day ago", "{} days ago", time as u32), time)
            .expect("Could not format translatable string")
    } else if duration.num_days() < 365 {
        let time = duration.num_days() / 30;
        formatx!(ngettext("{} month ago", "{} months ago", time as u32), time)
            .expect("Could not format translatable string")
    } else {
        let time = duration.num_days() / 365;
        formatx!(ngettext("{} year ago", "{} years ago", time as u32), time)
            .expect("Could not format translatable string")
    }
}

pub async fn texture(
    path: &impl AsRef<std::path::Path>,
) -> Result<gdk::Texture, image::error::ImageError> {
    let path = std::path::PathBuf::from(path.as_ref());
    crate::RUNTIME
        .spawn_blocking(move || {
            let image = image::ImageReader::open(path)?
                .with_guessed_format()?
                .decode()?;

            texture_from_image(image)
        })
        .await
        .expect("Could not spawn blocking to load image")
}

pub async fn texture_from_bytes<R: 'static + std::io::BufRead + std::io::Seek + Send>(
    cursor: R,
) -> Result<gdk::Texture, image::error::ImageError> {
    crate::RUNTIME
        .spawn_blocking(move || {
            let image = image::ImageReader::new(cursor)
                .with_guessed_format()?
                .decode()?;

            texture_from_image(image)
        })
        .await
        .expect("Could not spawn blocking to load image")
}

pub fn texture_from_image(
    image: image::DynamicImage,
) -> Result<gdk::Texture, image::error::ImageError> {
    let width = image.width() as i32;
    let height = image.height() as i32;
    // 3 == ["r", "g", "b"].len()
    let stride = (width * 3) as usize;
    let format = gdk::MemoryFormat::R8g8b8;

    let bytes = image.into_rgb8().into_vec();

    let texture = gdk::MemoryTexture::new(
        width,
        height,
        format,
        &glib::Bytes::from_owned(bytes),
        stride,
    );

    Ok(texture.upcast())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use podcasts_data::database::reset_db;
    use podcasts_data::dbqueries;
    use podcasts_data::pipeline::pipeline;
    use podcasts_data::utils::get_download_dir;
    use podcasts_data::{Save, Source};
    use std::fs;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_itunes_to_rss() -> Result<()> {
        let itunes_url = "https://itunes.apple.com/podcast/id1195206601";
        // they keep changing the urls
        let rss_url = "https://feeds.acast.com/public/shows/f5b64019-68c3-57d4-b70b-043e63e5cbf6";
        let rss_url2 = "https://rss.acast.com/intercepted-with-jeremy-scahill";
        let result_url = itunes_to_rss(itunes_url).await?;
        assert!(result_url == rss_url || result_url == rss_url2);

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
        // they keep changing the urls
        let rss_url = "https://feeds.acast.com/public/shows/f5b64019-68c3-57d4-b70b-043e63e5cbf6";
        let rss_url2 = "https://rss.acast.com/intercepted-with-jeremy-scahill";
        let result_url = itunes_lookup_id(id).await?;
        assert!(result_url == rss_url || result_url == rss_url2);

        let id = 000000000;
        assert!(itunes_lookup_id(id).await.is_err());
        Ok(())
    }

    #[tokio::test]
    #[ignore]
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

    #[tokio::test]
    #[ignore]
    async fn test_soundcloud_playlist_to_rss() -> Result<()> {
        // valid playlist
        let soundcloud_url =
            Url::parse("https://soundcloud.com/languagetransfer/sets/introduction-to-italian")?;
        let rss_url = String::from(
            "https://feeds.soundcloud.com/playlists/soundcloud:playlists:220248349/sounds.rss",
        );
        assert_eq!(
            Url::parse(&rss_url)?,
            soundcloud_to_rss(&soundcloud_url).await?
        );

        // invalid playlist link
        let soundcloud_url =
            Url::parse("https://soundcloud.com/languagetransfer/sets/does-not-exist")?;
        assert!(soundcloud_to_rss(&soundcloud_url).await.is_err());

        // user page with a playlist pinned at the top, should return user rss not playlist
        let soundcloud_url = Url::parse("https://soundcloud.com/yung-chomsky")?;
        let rss_url = String::from(
            "https://feeds.soundcloud.com/users/soundcloud:users:418603470/sounds.rss",
        );
        assert_eq!(
            Url::parse(&rss_url)?,
            soundcloud_to_rss(&soundcloud_url).await?
        );

        // playlist without rss entries
        let soundcloud_url =
            Url::parse("https://soundcloud.com/yung-chomsky/sets/music-for-podcasts-volume-1")?;
        let rss_url = String::from(
            "https://feeds.soundcloud.com/playlists/soundcloud:playlists:1165448311/sounds.rss",
        );
        assert_eq!(
            Url::parse(&rss_url)?,
            soundcloud_to_rss(&soundcloud_url).await?
        );
        Ok(())
    }

    #[test]
    #[ignore]
    fn should_refresh_cached_image_when_the_image_uri_changes() -> Result<()> {
        let _tempfile = reset_db()?;
        let mut original_feed = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        original_feed.push("resources/test/feeds/2018-01-20-LinuxUnplugged.xml");
        let original_url = format!(
            "{}{}",
            "file:/",
            fs::canonicalize(original_feed)?.to_str().unwrap()
        );
        println!("Made it here! (1)");
        let mut source = Source::from_url(&original_url)?;
        println!("Made it here! (2)");
        source.set_http_etag(None);
        source.set_last_modified(None);
        let sid = source.save()?.id();
        println!("Made it here! (3)");
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(vec![source]))?;
        println!("Made it here! (4)");
        println!("The source id is {}!", sid.0);
        dbqueries::get_sources().unwrap().iter().for_each(|s| {
            println!("{}:{}", s.id().0, s.uri());
        });

        let original = dbqueries::get_podcast_from_source_id(sid)?;
        println!("Made it here! (5)");
        let original_image_uri = original.image_uri();
        let original_image_uri_hash = original.image_uri_hash();
        let original_image_cached = original.image_cached();
        let download_dir = get_download_dir(original.title())?;
        let image_path = download_dir + "/cover.jpeg";
        let original_image_file_size = fs::metadata(&image_path)?.len(); // 693,343
        println!("Made it here! (6)");

        // Update the URI and refresh the feed
        let mut new_feed = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        new_feed.push("resources/test/feeds/2020-12-19-LinuxUnplugged.xml");
        let mut source = dbqueries::get_source_from_id(sid)?;
        let new_url = format!(
            "{}{}",
            "file:/",
            fs::canonicalize(new_feed)?.to_str().unwrap()
        );
        source.set_uri(new_url);
        source.set_http_etag(None);
        source.set_last_modified(None);
        source.save()?;
        println!("Made it here! (7)");
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(pipeline(vec![source]))?;

        println!("Made it here! (8)");
        let new = dbqueries::get_podcast_from_source_id(sid)?;
        let new_image_uri = new.image_uri();
        let new_image_uri_hash = new.image_uri_hash();
        let new_image_cached = new.image_cached();
        let new_image_file_size = fs::metadata(&image_path)?.len();

        println!("Made it here! (9)");
        assert_eq!(original.title(), new.title());
        assert_ne!(original_image_uri, new_image_uri);
        assert_ne!(original_image_uri_hash, new_image_uri_hash);
        assert_ne!(original_image_cached, new_image_cached);
        assert_ne!(original_image_file_size, new_image_file_size);

        fs::remove_file(image_path)?;
        Ok(())
    }

    #[test]
    fn test_format_relative_time() -> Result<()> {
        use chrono::Duration;
        assert_eq!("Just now", relative_time(Duration::seconds(5)));
        assert_eq!("1 minute ago", relative_time(Duration::seconds(60)));
        assert_eq!("5 minutes ago", relative_time(Duration::minutes(5)));
        assert_eq!("1 day ago", relative_time(Duration::days(1)));
        assert_eq!("5 days ago", relative_time(Duration::days(5)));
        assert_eq!("30 days ago", relative_time(Duration::days(30)));
        assert_eq!("1 month ago", relative_time(Duration::days(31)));
        assert_eq!("5 months ago", relative_time(Duration::days(30 * 5)));
        assert_eq!("11 months ago", relative_time(Duration::days(359)));
        assert_eq!("12 months ago", relative_time(Duration::days(360)));
        assert_eq!("1 year ago", relative_time(Duration::days(365)));
        assert_eq!("999 years ago", relative_time(Duration::days(365 * 999)));
        Ok(())
    }
}

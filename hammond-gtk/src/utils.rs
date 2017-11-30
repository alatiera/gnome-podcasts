use glib;
use gtk;
use gdk_pixbuf::Pixbuf;

use hammond_data::feed;
use hammond_data::{Podcast, Source};
use hammond_downloader::downloader;

use std::{thread, time};
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver};

use views::podcasts;

type Foo = RefCell<Option<(gtk::Stack, Receiver<bool>)>>;

// Create a thread local storage that will store the arguments to be transfered.
thread_local!(static GLOBAL: Foo = RefCell::new(None));

/// Update the rss feed(s) originating from `Source`.
/// If `source` is None, Fetches all the `Source` entries in the database and updates them.
/// `delay` represents the desired time in seconds for the thread to sleep before executing.
/// When It's done,it queues up a `podcast_view` refresh.
pub fn refresh_feed(stack: &gtk::Stack, source: Option<Vec<Source>>, delay: Option<u64>) {
    // Create a async channel.
    let (sender, receiver) = channel();

    // Pass the desired arguments into the Local Thread Storage.
    GLOBAL.with(clone!(stack => move |global| {
        *global.borrow_mut() = Some((stack, receiver));
    }));

    thread::spawn(move || {
        if let Some(s) = delay {
            let t = time::Duration::from_secs(s);
            thread::sleep(t);
        }

        let feeds = {
            if let Some(vec) = source {
                Ok(feed::fetch(vec))
            } else {
                feed::fetch_all()
            }
        };

        if let Ok(x) = feeds {
            feed::index(x);

            sender.send(true).expect("Couldn't send data to channel");;
            glib::idle_add(refresh_podcasts_view);
        };
    });
}

fn refresh_podcasts_view() -> glib::Continue {
    GLOBAL.with(|global| {
        if let Some((ref stack, ref reciever)) = *global.borrow() {
            if reciever.try_recv().is_ok() {
                podcasts::update_podcasts_view(stack);
            }
        }
    });
    glib::Continue(false)
}

pub fn get_pixbuf_from_path(pd: &Podcast) -> Option<Pixbuf> {
    let img_path = downloader::cache_image(pd);
    if let Some(i) = img_path {
        Pixbuf::new_from_file_at_scale(&i, 256, 256, true).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use hammond_data::Source;
    use hammond_data::feed::index;
    use hammond_data::dbqueries;
    use diesel::Identifiable;
    use super::*;

    #[test]
    fn test_get_pixbuf_from_path() {
        let url = "http://www.newrustacean.com/feed.xml";

        // Create and index a source
        let source = Source::from_url(url).unwrap();
        // Copy it's id
        let sid = source.id().clone();

        // Convert Source it into a Feed and index it
        let feed = source.into_feed().unwrap();
        index(vec![feed]);

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid).unwrap();
        let pxbuf = get_pixbuf_from_path(&pd);
        assert!(pxbuf.is_some());
    }
}

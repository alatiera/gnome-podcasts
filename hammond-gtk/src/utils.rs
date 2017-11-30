use glib;
use gtk;

use hammond_data::feed;
use hammond_data::Source;

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

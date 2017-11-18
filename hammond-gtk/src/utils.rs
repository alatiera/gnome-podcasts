use glib;
use gtk;

use hammond_data::index_feed;
use hammond_data::models::Source;
use hammond_data::Database;

use std::{thread, time};
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver};

use views::podcasts_view;

type Foo = RefCell<Option<(Database, gtk::Stack, Receiver<bool>)>>;

// Create a thread local storage that will store the arguments to be transfered.
thread_local!(static GLOBAL: Foo = RefCell::new(None));

/// Update the rss feed(s) originating from `Source`.
/// If `source` is None, Fetches all the `Source` entries in the database and updates them.
/// `delay` represents the desired time in seconds for the thread to sleep before executing.
/// When It's done,it queues up a `podcast_view` refresh.
pub fn refresh_feed(
    db: &Database,
    stack: &gtk::Stack,
    source: Option<Vec<Source>>,
    delay: Option<u64>,
) {
    // Create a async channel.
    let (sender, receiver) = channel();

    // Pass the desired arguments into the Local Thread Storage.
    GLOBAL.with(clone!(db, stack => move |global| {
        *global.borrow_mut() = Some((db, stack, receiver));
    }));

    thread::spawn(clone!(db => move || {
        if let Some(s) = delay{
            let t = time::Duration::from_secs(s);
            thread::sleep(t);
        }

        let feeds = {
            if let Some(mut vec) = source {
                Ok(index_feed::fetch(&db, vec))
            } else {
                index_feed::fetch_all(&db)
            }
        };

        if let Ok(mut x) = feeds {
            index_feed::index(&db, &mut x);
            info!("Indexing done.");

            sender.send(true).expect("Couldn't send data to channel");;
            glib::idle_add(refresh_podcasts_view);
        };
    }));
}

fn refresh_podcasts_view() -> glib::Continue {
    GLOBAL.with(|global| {
        if let Some((ref db, ref stack, ref reciever)) = *global.borrow() {
            if reciever.try_recv().is_ok() {
                podcasts_view::update_podcasts_view(db, stack);
            }
        }
    });
    glib::Continue(false)
}

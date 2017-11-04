use glib;
use gtk;
use rayon::prelude::*;

use hammond_data::index_feed;
use hammond_data::models::Source;
use hammond_data::index_feed::Database;

use std::thread;
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver};

use views::podcasts_view;

// Create a thread local storage that will store the arguments to be transfered.
thread_local!(
    static GLOBAL: RefCell<Option<(Database,
    gtk::Stack,
    Receiver<bool>)>> = RefCell::new(None));

pub fn refresh_feed(db: &Database, stack: &gtk::Stack, source: Option<Box<Vec<Source>>>) {
    let (sender, receiver) = channel();

    GLOBAL.with(clone!(db, stack => move |global| {
        *global.borrow_mut() = Some((db, stack, receiver));
    }));

    // TODO: add timeout option and error reporting.
    thread::spawn(clone!(db => move || {
        let feeds = {
            if let Some(mut boxed_vec) = source {
                let f = boxed_vec
                    .par_iter_mut()
                    .filter_map(|mut s| {
                        index_feed::refresh_source(&db, &mut s).ok()
                    })
                    .collect();
                Ok(f)
            } else {
                index_feed::fetch_feeds(&db)
            }
        };

        if let Ok(mut x) = feeds {
            index_feed::index_feed(&db, &mut x);

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

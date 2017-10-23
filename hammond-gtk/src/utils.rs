use glib;
use gtk;
// use gtk::prelude::*;

use hammond_data;
use hammond_data::index_feed::Feed;
use hammond_data::models::Source;
use hammond_data::index_feed::Database;

use std::thread;
use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver};

use views::podcasts_view;

// http://gtk-rs.org/tuto/closures
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

// Create a thread local storage that will store the arguments to be transfered.
thread_local!(
    static GLOBAL: RefCell<Option<(Database,
    gtk::Stack,
    Receiver<bool>)>> = RefCell::new(None));

pub fn refresh_db(db: &Database, stack: &gtk::Stack) {
    // Create a async channel.
    let (sender, receiver) = channel();

    // Pass the desired arguments into the Local Thread Storage.
    GLOBAL.with(clone!(db, stack => move |global| {
        *global.borrow_mut() = Some((db, stack, receiver));
    }));

    // The implementation of how this is done is probably terrible but it works!.
    // TODO: add timeout option and error reporting.
    thread::spawn(clone!(db => move || {
        let t = hammond_data::index_feed::index_loop(&db);
        if t.is_err() {
            error!("Error While trying to update the database.");
            error!("Error msg: {}", t.unwrap_err());
        };
        sender.send(true).expect("Couldn't send data to channel");;

        // http://gtk-rs.org/docs/glib/source/fn.idle_add.html
        glib::idle_add(refresh_podcasts_view);
    }));
}

pub fn refresh_feed(db: &Database, stack: &gtk::Stack, source: &mut Source) {
    let (sender, receiver) = channel();

    GLOBAL.with(clone!(db, stack => move |global| {
        *global.borrow_mut() = Some((db, stack, receiver));
    }));

    let mut source = source.clone();
    // TODO: add timeout option and error reporting.
    thread::spawn(clone!(db => move || {
        let foo_ = hammond_data::index_feed::refresh_source(&db, &mut source);

        if let Ok(x) = foo_ {
            let Feed(mut req, s) = x;
            let s = hammond_data::index_feed::complete_index_from_source(&mut req, &s, &db);
            if s.is_err() {
                error!("Error While trying to update the database.");
                error!("Error msg: {}", s.unwrap_err());
            };

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

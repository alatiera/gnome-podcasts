use gtk;
use gtk::prelude::*;

use index_feed;
use hammond_data::index_feed::Database;
use utils;

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

pub fn get_headerbar(db: &Database, stack: &gtk::Stack) -> gtk::HeaderBar {
    let builder = include_str!("../gtk/headerbar.ui");
    let builder = gtk::Builder::new_from_string(builder);

    let header: gtk::HeaderBar = builder.get_object("headerbar1").unwrap();
    let home_button: gtk::Button = builder.get_object("homebutton").unwrap();
    let refresh_button: gtk::Button = builder.get_object("refbutton").unwrap();
    let _search_button: gtk::Button = builder.get_object("searchbutton").unwrap();

    let add_toggle_button: gtk::MenuButton = builder.get_object("add-toggle-button").unwrap();
    let add_popover: gtk::Popover = builder.get_object("add-popover").unwrap();
    let new_url: gtk::Entry = builder.get_object("new-url").unwrap();
    let add_button: gtk::Button = builder.get_object("add-button").unwrap();
    // TODO: check if url exists in the db and lock the button
    new_url.connect_changed(move |url| {
        println!("{:?}", url.get_text());
    });

    add_button.connect_clicked(clone!(db, stack, add_popover => move |_| {
        let url = new_url.get_text().unwrap_or_default();
        on_add_bttn_clicked(&db, &stack, &url);

        // TODO: lock the button instead of hiding and add notification of feed added.
        // TODO: map the spinner
        add_popover.hide();
    }));
    add_popover.hide();
    add_toggle_button.set_popover(&add_popover);

    // TODO: make it a back arrow button, that will hide when appropriate,
    // and add a StackSwitcher when more views are added.
    home_button.connect_clicked(clone!(stack => move |_| stack.set_visible_child_name("pd_grid")));

    // FIXME: There appears to be a memmory leak here.
    refresh_button.connect_clicked(clone!(stack, db => move |_| {
        utils::refresh_db(&db, &stack);
    }));

    header
}

fn on_add_bttn_clicked(db: &Database, stack: &gtk::Stack, url: &str) {
    let source = {
        let tempdb = db.lock().unwrap();
        index_feed::insert_return_source(&tempdb, url)
    };
    info!("{:?} feed added", url);

    if let Ok(mut s) = source {
        // update the db
        utils::refresh_feed(db, stack, &mut s);
    } else {
        error!("Expected Error, feed probably already exists.");
        error!("Error: {:?}", source.unwrap_err());
    }
}

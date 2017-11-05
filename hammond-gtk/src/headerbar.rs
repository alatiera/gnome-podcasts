use gtk;
use gtk::prelude::*;

use hammond_data::index_feed;
use hammond_data::index_feed::Database;

use podcasts_view::update_podcasts_view;
use utils;

pub fn get_headerbar(db: &Database, stack: &gtk::Stack) -> gtk::HeaderBar {
    let builder = gtk::Builder::new_from_string(include_str!("../gtk/headerbar.ui"));

    let header: gtk::HeaderBar = builder.get_object("headerbar1").unwrap();
    let home_button: gtk::Button = builder.get_object("homebutton").unwrap();
    let refresh_button: gtk::Button = builder.get_object("refbutton").unwrap();

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
    home_button.connect_clicked(clone!(db, stack => move |_| {
        let vis = stack.get_visible_child_name().unwrap();
        if vis == "fb_parent" {
            // More conviniet way to reload podcasts_flowbox while trying out stuff.
            // Ideally, the functionality should be removed from final design.
            update_podcasts_view(&db, &stack);
        } else {
            stack.set_visible_child_name("fb_parent");
        }
    }));

    // FIXME: There appears to be a memmory leak here.
    refresh_button.connect_clicked(clone!(stack, db => move |_| {
        utils::refresh_feed(&db, &stack, None, None);
    }));

    header
}

fn on_add_bttn_clicked(db: &Database, stack: &gtk::Stack, url: &str) {
    let source = {
        let tempdb = db.lock().unwrap();
        index_feed::insert_return_source(&tempdb, url)
    };
    info!("{:?} feed added", url);

    if let Ok(s) = source {
        // update the db
        utils::refresh_feed(db, stack, Some(vec![s]), None);
    } else {
        error!("Expected Error, feed probably already exists.");
        error!("Error: {:?}", source.unwrap_err());
    }
}

use gtk;
use gtk::prelude::*;

use hammond_data::models::NewSource;
use hammond_data::utils::url_cleaner;

use podcasts_view::update_podcasts_view;
use utils;

pub fn get_headerbar(stack: &gtk::Stack) -> gtk::HeaderBar {
    let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

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

    add_button.connect_clicked(clone!(stack, add_popover => move |_| {
        let url = new_url.get_text().unwrap_or_default();
        let url = url_cleaner(&url);
        on_add_bttn_clicked(&stack, &url);

        // TODO: lock the button instead of hiding and add notification of feed added.
        // TODO: map the spinner
        add_popover.hide();
    }));
    add_popover.hide();
    add_toggle_button.set_popover(&add_popover);

    // TODO: make it a back arrow button, that will hide when appropriate,
    // and add a StackSwitcher when more views are added.
    home_button.connect_clicked(clone!(stack => move |_| {
        let vis = stack.get_visible_child_name().unwrap();
        stack.set_visible_child_name("fb_parent");
        if vis != "pdw" {
            update_podcasts_view(&stack);
        }
    }));

    // FIXME: There appears to be a memmory leak here.
    refresh_button.connect_clicked(clone!(stack => move |_| {
        utils::refresh_feed(&stack, None, None);
    }));

    header
}

fn on_add_bttn_clicked(stack: &gtk::Stack, url: &str) {
    let source = NewSource::new_with_uri(url).into_source();
    info!("{:?} feed added", url);

    if let Ok(s) = source {
        // update the db
        utils::refresh_feed(stack, Some(vec![s]), None);
    } else {
        error!("Feed probably already exists.");
        error!("Error: {:?}", source.unwrap_err());
    }
}

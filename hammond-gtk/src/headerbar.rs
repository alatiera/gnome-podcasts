use gtk;
use gtk::prelude::*;

use hammond_data::Source;
use hammond_data::utils::url_cleaner;

use std::rc::Rc;

use utils;
use content::Content;

#[derive(Debug)]
pub struct Header {
    pub container: gtk::HeaderBar,
    refresh: gtk::Button,
    add_toggle: gtk::MenuButton,
    switch: gtk::StackSwitcher,
}

impl Header {
    pub fn new() -> Header {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let header: gtk::HeaderBar = builder.get_object("headerbar1").unwrap();
        let refresh: gtk::Button = builder.get_object("refbutton").unwrap();
        let add_toggle: gtk::MenuButton = builder.get_object("add-toggle-button").unwrap();
        let switch: gtk::StackSwitcher = builder.get_object("switch").unwrap();
        switch.set_halign(gtk::Align::Center);
        switch.show();

        Header {
            container: header,
            refresh,
            add_toggle,
            switch,
        }
    }

    pub fn new_initialized(content: Rc<Content>) -> Header {
        let header = Header::new();
        header.init(content);
        header
    }

    fn init(&self, content: Rc<Content>) {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let add_popover: gtk::Popover = builder.get_object("add-popover").unwrap();
        let new_url: gtk::Entry = builder.get_object("new-url").unwrap();
        let add_button: gtk::Button = builder.get_object("add-button").unwrap();
        self.switch.set_stack(&content.stack);

        new_url.connect_changed(move |url| {
            println!("{:?}", url.get_text());
        });

        add_button.connect_clicked(clone!(content, add_popover, new_url => move |_| {
            on_add_bttn_clicked(content.clone(), &new_url);

            // TODO: lock the button instead of hiding and add notification of feed added.
            // TODO: map the spinner
            add_popover.hide();
        }));
        self.add_toggle.set_popover(&add_popover);

        // FIXME: There appears to be a memmory leak here.
        self.refresh.connect_clicked(move |_| {
            utils::refresh_feed(content.clone(), None, None);
        });
    }
}

fn on_add_bttn_clicked(content: Rc<Content>, entry: &gtk::Entry) {
    let url = entry.get_text().unwrap_or_default();
    let url = url_cleaner(&url);
    let source = Source::from_url(&url);

    if let Ok(s) = source {
        info!("{:?} feed added", url);
        // update the db
        utils::refresh_feed(content, Some(vec![s]), None);
    } else {
        error!("Feed probably already exists.");
        error!("Error: {:?}", source.unwrap_err());
    }
}

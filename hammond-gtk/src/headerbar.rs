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
    add_toggle: gtk::MenuButton,
    switch: gtk::StackSwitcher,
    back_button: gtk::Button,
    show_title: gtk::Label,
}

impl Default for Header {
    fn default() -> Header {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let header: gtk::HeaderBar = builder.get_object("headerbar").unwrap();
        let add_toggle: gtk::MenuButton = builder.get_object("add_toggle_button").unwrap();
        let switch: gtk::StackSwitcher = builder.get_object("switch").unwrap();
        let back_button: gtk::Button = builder.get_object("back_button").unwrap();
        let show_title: gtk::Label = builder.get_object("show_title").unwrap();

        switch.set_halign(gtk::Align::Center);
        switch.show();

        Header {
            container: header,
            add_toggle,
            switch,
            back_button,
            show_title,
        }
    }
}

impl Header {
    #[allow(dead_code)]
    pub fn new(content: Rc<Content>) -> Rc<Header> {
        let h = Header::default();
        h.init(content);
        Rc::new(h)
    }

    pub fn init(&self, content: Rc<Content>) {
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

        let switch = &self.switch;
        let add_toggle = &self.add_toggle;
        let show_title = &self.show_title;
        self.back_button.connect_clicked(
            clone!(content, switch, add_toggle, show_title => move |back| {
            switch.show();
            add_toggle.show();
            back.hide();
            show_title.hide();
            content.shows.stack.set_visible_child_full("podcasts", gtk::StackTransitionType::SlideRight);
        }),
        );
    }

    pub fn switch_to_back(&self, title: &str) {
        self.switch.hide();
        self.add_toggle.hide();
        self.back_button.show();
        self.set_show_title(title);
        self.show_title.show();
    }

    pub fn switch_to_normal(&self) {
        self.switch.show();
        self.add_toggle.show();
        self.back_button.hide();
        self.show_title.hide();
    }

    pub fn set_show_title(&self, title: &str) {
        self.show_title.set_text(title)
    }
}

fn on_add_bttn_clicked(content: Rc<Content>, entry: &gtk::Entry) {
    let url = entry.get_text().unwrap_or_default();
    let url = url_cleaner(&url);
    let source = Source::from_url(&url);

    if let Ok(s) = source {
        info!("{:?} feed added", url);
        // update the db
        utils::refresh_feed(content, Some(vec![s]));
    } else {
        error!("Feed probably already exists.");
        error!("Error: {:?}", source.unwrap_err());
    }
}

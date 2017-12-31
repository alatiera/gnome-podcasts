use gtk;
use gtk::prelude::*;

use hammond_data::Source;

use std::rc::Rc;

use utils;
use content::Content;

#[derive(Debug, Clone)]
pub struct Header {
    pub container: gtk::HeaderBar,
    add_toggle: gtk::MenuButton,
    switch: gtk::StackSwitcher,
    back_button: gtk::Button,
    show_title: gtk::Label,
    update_box: gtk::Box,
    update_label: gtk::Label,
    update_spinner: gtk::Spinner,
}

impl Default for Header {
    fn default() -> Header {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let header: gtk::HeaderBar = builder.get_object("headerbar").unwrap();
        let add_toggle: gtk::MenuButton = builder.get_object("add_toggle").unwrap();
        let switch: gtk::StackSwitcher = builder.get_object("switch").unwrap();
        let back_button: gtk::Button = builder.get_object("back_button").unwrap();
        let show_title: gtk::Label = builder.get_object("show_title").unwrap();
        let update_box: gtk::Box = builder.get_object("update_notification").unwrap();
        let update_label: gtk::Label = builder.get_object("update_label").unwrap();
        let update_spinner: gtk::Spinner = builder.get_object("update_spinner").unwrap();

        Header {
            container: header,
            add_toggle,
            switch,
            back_button,
            show_title,
            update_box,
            update_label,
            update_spinner,
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

        let add_popover: gtk::Popover = builder.get_object("add_popover").unwrap();
        let new_url: gtk::Entry = builder.get_object("new_url").unwrap();
        let add_button: gtk::Button = builder.get_object("add_button").unwrap();
        self.switch.set_stack(&content.get_stack());

        new_url.connect_changed(move |url| {
            println!("{:?}", url.get_text());
        });

        let header = Rc::new(self.clone());
        add_button.connect_clicked(clone!(content, header, add_popover, new_url => move |_| {
            on_add_bttn_clicked(content.clone(), header.clone(), &new_url);
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
            content.get_shows().get_stack().set_visible_child_full("podcasts", gtk::StackTransitionType::SlideRight);
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

    pub fn show_update_notification(&self) {
        self.update_spinner.start();
        self.update_box.show();
        self.update_spinner.show();
        self.update_label.show();
    }

    pub fn hide_update_notification(&self) {
        self.update_spinner.stop();
        self.update_box.hide();
        self.update_spinner.hide();
        self.update_label.hide();
    }
}

fn on_add_bttn_clicked(content: Rc<Content>, headerbar: Rc<Header>, entry: &gtk::Entry) {
    let url = entry.get_text().unwrap_or_default();
    let source = Source::from_url(&url);

    if let Ok(s) = source {
        info!("{:?} feed added", url);
        // update the db
        utils::refresh_feed(content, headerbar, Some(vec![s]));
    } else {
        error!("Feed probably already exists.");
        error!("Error: {:?}", source.unwrap_err());
    }
}

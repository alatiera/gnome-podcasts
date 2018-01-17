use gtk;
use gtk::prelude::*;

use hammond_data::Source;

use std::sync::Arc;
use std::sync::mpsc::Sender;

use app::Action;
use content::Content;

#[derive(Debug, Clone)]
pub struct Header {
    pub container: gtk::HeaderBar,
    add_toggle: gtk::MenuButton,
    switch: gtk::StackSwitcher,
    back_button: gtk::Button,
    show_title: gtk::Label,
    update_button: gtk::ModelButton,
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
        let update_button: gtk::ModelButton = builder.get_object("update_button").unwrap();
        let update_box: gtk::Box = builder.get_object("update_notification").unwrap();
        let update_label: gtk::Label = builder.get_object("update_label").unwrap();
        let update_spinner: gtk::Spinner = builder.get_object("update_spinner").unwrap();

        Header {
            container: header,
            add_toggle,
            switch,
            back_button,
            show_title,
            update_button,
            update_box,
            update_label,
            update_spinner,
        }
    }
}

impl Header {
    #[allow(dead_code)]
    pub fn new(content: Arc<Content>, sender: Sender<Action>) -> Arc<Header> {
        let h = Header::default();
        h.init(content, sender);
        Arc::new(h)
    }

    pub fn init(&self, content: Arc<Content>, sender: Sender<Action>) {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let add_popover: gtk::Popover = builder.get_object("add_popover").unwrap();
        let new_url: gtk::Entry = builder.get_object("new_url").unwrap();
        let add_button: gtk::Button = builder.get_object("add_button").unwrap();
        self.switch.set_stack(&content.get_stack());

        new_url.connect_changed(move |url| {
            println!("{:?}", url.get_text());
        });

        add_button.connect_clicked(clone!(add_popover, new_url, sender => move |_| {
            on_add_bttn_clicked(&new_url, sender.clone());
            add_popover.hide();
        }));

        self.add_toggle.set_popover(&add_popover);

        self.update_button.connect_clicked(move |_| {
            sender.send(Action::UpdateSources(None)).unwrap();
        });

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

fn on_add_bttn_clicked(entry: &gtk::Entry, sender: Sender<Action>) {
    let url = entry.get_text().unwrap_or_default();
    let source = Source::from_url(&url);

    if source.is_ok() {
        sender.send(Action::UpdateSources(source.ok())).unwrap();
    } else {
        error!("Something went wrong.");
        error!("Error: {:?}", source.unwrap_err());
    }
}

use gtk;
use gtk::prelude::*;

use hammond_data::Source;
use hammond_data::utils::url_cleaner;

use views::podcasts::update_podcasts_view;
use utils;

#[derive(Debug)]
pub struct Header {
    pub container: gtk::HeaderBar,
    home: gtk::Button,
    refresh: gtk::Button,
    add_toggle: gtk::MenuButton,
}

impl Header {
    pub fn new() -> Header {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let header: gtk::HeaderBar = builder.get_object("headerbar1").unwrap();
        let home: gtk::Button = builder.get_object("homebutton").unwrap();
        let refresh: gtk::Button = builder.get_object("refbutton").unwrap();
        let add_toggle: gtk::MenuButton = builder.get_object("add-toggle-button").unwrap();

        Header {
            container: header,
            home,
            refresh,
            add_toggle,
        }
    }

    pub fn new_initialized(stack: &gtk::Stack) -> Header {
        let header = Header::new();
        header.init(stack);
        header
    }

    fn init(&self, stack: &gtk::Stack) {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let add_popover: gtk::Popover = builder.get_object("add-popover").unwrap();
        let new_url: gtk::Entry = builder.get_object("new-url").unwrap();
        let add_button: gtk::Button = builder.get_object("add-button").unwrap();

        new_url.connect_changed(move |url| {
            println!("{:?}", url.get_text());
        });

        add_button.connect_clicked(clone!(stack, add_popover, new_url => move |_| {
            on_add_bttn_clicked(&stack, &new_url);

            // TODO: lock the button instead of hiding and add notification of feed added.
            // TODO: map the spinner
            add_popover.hide();
        }));
        self.add_toggle.set_popover(&add_popover);

        // TODO: make it a back arrow button, that will hide when appropriate,
        // and add a StackSwitcher when more views are added.
        self.home.connect_clicked(clone!(stack => move |_| {
            let vis = stack.get_visible_child_name().unwrap();
            stack.set_visible_child_name("fb_parent");
            if vis != "pdw" {
                update_podcasts_view(&stack);
            }
        }));

        // FIXME: There appears to be a memmory leak here.
        self.refresh.connect_clicked(clone!(stack => move |_| {
            utils::refresh_feed(&stack, None, None);
        }));
    }
}

fn on_add_bttn_clicked(stack: &gtk::Stack, entry: &gtk::Entry) {
    let url = entry.get_text().unwrap_or_default();
    let url = url_cleaner(&url);
    let source = Source::from_url(&url);

    if let Ok(s) = source {
        info!("{:?} feed added", url);
        // update the db
        utils::refresh_feed(stack, Some(vec![s]), None);
    } else {
        error!("Feed probably already exists.");
        error!("Error: {:?}", source.unwrap_err());
    }
}

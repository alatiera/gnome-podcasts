use gio::MenuModel;
use glib;
use gtk;
use gtk::prelude::*;

use failure::Error;
use failure::ResultExt;
use url::Url;

use hammond_data::{dbqueries, Source};

use std::sync::mpsc::Sender;

use app::Action;
use stacks::Content;
use utils::{itunes_to_rss, refresh};

#[derive(Debug, Clone)]
// TODO: split this into smaller
pub struct Header {
    container: gtk::HeaderBar,
    add_toggle: gtk::MenuButton,
    switch: gtk::StackSwitcher,
    back: gtk::Button,
    show_title: gtk::Label,
    update_box: gtk::Box,
    update_label: gtk::Label,
    update_spinner: gtk::Spinner,
    menu_popover: gtk::Popover,
    app_menu: MenuModel,
}

impl Default for Header {
    fn default() -> Header {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/headerbar.ui");

        let header = builder.get_object("headerbar").unwrap();
        let add_toggle = builder.get_object("add_toggle").unwrap();
        let switch = builder.get_object("switch").unwrap();
        let back = builder.get_object("back").unwrap();
        let show_title = builder.get_object("show_title").unwrap();
        let update_box = builder.get_object("update_notification").unwrap();
        let update_label = builder.get_object("update_label").unwrap();
        let update_spinner = builder.get_object("update_spinner").unwrap();
        let menu_popover = builder.get_object("menu_popover").unwrap();
        let menus = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/menus.ui");
        let app_menu = menus.get_object("app-menu").unwrap();

        Header {
            container: header,
            add_toggle,
            switch,
            back,
            show_title,
            update_box,
            update_label,
            update_spinner,
            menu_popover,
            app_menu,
        }
    }
}

// TODO: Refactor components into smaller state machines
impl Header {
    pub fn new(
        content: &Content,
        window: &gtk::ApplicationWindow,
        sender: &Sender<Action>,
    ) -> Header {
        let h = Header::default();
        h.init(content, window, &sender);
        h
    }

    pub fn init(
        &self,
        content: &Content,
        window: &gtk::ApplicationWindow,
        sender: &Sender<Action>,
    ) {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/headerbar.ui");

        let add_popover: gtk::Popover = builder.get_object("add_popover").unwrap();
        let new_url: gtk::Entry = builder.get_object("new_url").unwrap();
        let add_button: gtk::Button = builder.get_object("add_button").unwrap();
        let result_label: gtk::Label = builder.get_object("result_label").unwrap();
        self.switch.set_stack(&content.get_stack());

        new_url.connect_changed(clone!(add_button => move |url| {
            on_url_change(url, &result_label, &add_button)
                .map_err(|err| error!("Error: {}", err))
                .ok();
        }));

        add_button.connect_clicked(clone!(add_popover, new_url, sender => move |_| {
            on_add_bttn_clicked(&new_url, sender.clone())
                .map_err(|err| error!("Error: {}", err))
                .ok();
            add_popover.hide();
        }));

        self.add_toggle.set_popover(&add_popover);

        // Add the Headerbar to the window.
        window.set_titlebar(&self.container);

        let switch = &self.switch;
        let add_toggle = &self.add_toggle;
        let show_title = &self.show_title;
        self.back.connect_clicked(
            clone!(switch, add_toggle, show_title, sender => move |back| {
                switch.show();
                add_toggle.show();
                back.hide();
                show_title.hide();
                sender.send(Action::ShowShowsAnimated)
                    .map_err(|err| error!("Action Sender: {}", err))
                    .ok();
            }),
        );

        self.menu_popover.bind_model(Some(&self.app_menu), None);
    }

    pub fn switch_to_back(&self, title: &str) {
        self.switch.hide();
        self.add_toggle.hide();
        self.back.show();
        self.set_show_title(title);
        self.show_title.show();
    }

    pub fn switch_to_normal(&self) {
        self.switch.show();
        self.add_toggle.show();
        self.back.hide();
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

// FIXME: THIS ALSO SUCKS!
fn on_add_bttn_clicked(entry: &gtk::Entry, sender: Sender<Action>) -> Result<(), Error> {
    let url = entry.get_text().unwrap_or_default();
    let url = if url.contains("itunes.com") || url.contains("apple.com") {
        info!("Detected itunes url.");
        let foo = itunes_to_rss(&url)?;
        info!("Resolved to {}", foo);
        foo
    } else {
        url.to_owned()
    };

    let source = Source::from_url(&url).context("Failed to convert url to a Source entry.")?;
    entry.set_text("");

    gtk::idle_add(move || {
        refresh(Some(vec![source.clone()]), sender.clone());
        glib::Continue(false)
    });
    Ok(())
}

// FIXME: THIS SUCKS!
fn on_url_change(
    entry: &gtk::Entry,
    result: &gtk::Label,
    add_button: &gtk::Button,
) -> Result<(), Error> {
    let uri = entry
        .get_text()
        .ok_or_else(|| format_err!("GtkEntry blew up somehow."))?;
    debug!("Url: {}", uri);

    let url = Url::parse(&uri);
    // TODO: refactor to avoid duplication
    match url {
        Ok(u) => {
            if !dbqueries::source_exists(u.as_str())? {
                add_button.set_sensitive(true);
                result.hide();
                result.set_label("");
            } else {
                add_button.set_sensitive(false);
                result.set_label("Show already exists.");
                result.show();
            }
            Ok(())
        }
        Err(err) => {
            add_button.set_sensitive(false);
            if !uri.is_empty() {
                result.set_label("Invalid url.");
                result.show();
                error!("Error: {}", err);
            } else {
                result.hide();
            }
            Ok(())
        }
    }
}

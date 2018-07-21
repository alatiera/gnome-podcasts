use gio::MenuModel;
use gtk;
use gtk::prelude::*;

use crossbeam_channel::Sender;
use failure::Error;
use rayon;
use url::Url;

use hammond_data::{dbqueries, Source};

use app::Action;
use stacks::Content;
use utils::{itunes_to_rss, refresh};

use std::rc::Rc;

#[derive(Debug, Clone)]
// TODO: Factor out the hamburger menu
// TODO: Make a proper state machine for the headerbar states
pub struct Header {
    pub container: gtk::HeaderBar,
    switch: gtk::StackSwitcher,
    back: gtk::Button,
    show_title: gtk::Label,
    menu_button: gtk::MenuButton,
    app_menu: MenuModel,
    updater: UpdateIndicator,
    add: AddPopover,
}

#[derive(Debug, Clone)]
struct UpdateIndicator {
    container: gtk::Box,
    text: gtk::Label,
    spinner: gtk::Spinner,
}

impl UpdateIndicator {
    fn show(&self) {
        self.spinner.start();
        self.spinner.show();
        self.container.show();
        self.text.show();
    }

    fn hide(&self) {
        self.spinner.stop();
        self.spinner.hide();
        self.container.hide();
        self.text.hide();
    }
}

#[derive(Debug, Clone)]
struct AddPopover {
    container: gtk::Popover,
    result: gtk::Label,
    entry: gtk::Entry,
    add: gtk::Button,
    toggle: gtk::MenuButton,
}

impl AddPopover {
    // FIXME: THIS ALSO SUCKS!
    fn on_add_clicked(&self, sender: &Sender<Action>) -> Result<(), Error> {
        let url = self
            .entry
            .get_text()
            .ok_or_else(|| format_err!("GtkEntry blew up somehow."))?;

        debug!("Url: {}", url);
        let url = if url.contains("itunes.com") || url.contains("apple.com") {
            info!("Detected itunes url.");
            let foo = itunes_to_rss(&url)?;
            info!("Resolved to {}", foo);
            foo
        } else {
            url.to_owned()
        };

        self.entry.set_text("");

        rayon::spawn(clone!(sender => move || {
            if let Ok(source) = Source::from_url(&url) {
                refresh(Some(vec![source]), sender.clone());
            } else {
                error!("Failed to convert, url: {}, to a source entry", url);
            }
        }));

        self.container.hide();
        Ok(())
    }

    // FIXME: THIS SUCKS! REFACTOR ME.
    fn on_entry_changed(&self) -> Result<(), Error> {
        let uri = self
            .entry
            .get_text()
            .ok_or_else(|| format_err!("GtkEntry blew up somehow."))?;
        debug!("Url: {}", uri);

        let url = Url::parse(&uri);
        // TODO: refactor to avoid duplication
        match url {
            Ok(u) => {
                if !dbqueries::source_exists(u.as_str())? {
                    self.add.set_sensitive(true);
                    self.result.hide();
                    self.result.set_label("");
                } else {
                    self.add.set_sensitive(false);
                    self.result.set_label("Show already exists.");
                    self.result.show();
                }
                Ok(())
            }
            Err(err) => {
                self.add.set_sensitive(false);
                if !uri.is_empty() {
                    self.result.set_label("Invalid url.");
                    self.result.show();
                    error!("Error: {}", err);
                } else {
                    self.result.hide();
                }
                Ok(())
            }
        }
    }
}

impl Default for Header {
    fn default() -> Header {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/headerbar.ui");
        let menus = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/hamburger.ui");

        let header = builder.get_object("headerbar").unwrap();
        let switch = builder.get_object("switch").unwrap();
        let back = builder.get_object("back").unwrap();
        let show_title = builder.get_object("show_title").unwrap();
        let menu_button = builder.get_object("menu_button").unwrap();
        let app_menu = menus.get_object("menu").unwrap();

        let update_box = builder.get_object("update_notification").unwrap();
        let update_label = builder.get_object("update_label").unwrap();
        let update_spinner = builder.get_object("update_spinner").unwrap();

        let updater = UpdateIndicator {
            container: update_box,
            text: update_label,
            spinner: update_spinner,
        };

        let add_toggle = builder.get_object("add_toggle").unwrap();
        let add_popover = builder.get_object("add_popover").unwrap();
        let new_url = builder.get_object("new_url").unwrap();
        let add_button = builder.get_object("add_button").unwrap();
        let result = builder.get_object("result_label").unwrap();
        let add = AddPopover {
            container: add_popover,
            entry: new_url,
            toggle: add_toggle,
            add: add_button,
            result,
        };

        Header {
            container: header,
            switch,
            back,
            show_title,
            menu_button,
            app_menu,
            updater,
            add,
        }
    }
}

// TODO: Factor out the hamburger menu
// TODO: Make a proper state machine for the headerbar states
impl Header {
    pub fn new(content: &Content, sender: &Sender<Action>) -> Rc<Self> {
        let h = Rc::new(Header::default());
        Self::init(&h, content, &sender);
        h
    }

    pub fn init(s: &Rc<Self>, content: &Content, sender: &Sender<Action>) {
        let weak = Rc::downgrade(s);

        s.switch.set_stack(&content.get_stack());

        s.add.entry.connect_changed(clone!(weak => move |_| {
            weak.upgrade().map(|h| {
                h.add.on_entry_changed()
                    .map_err(|err| error!("Error: {}", err))
                    .ok();
            });
        }));

        s.add.add.connect_clicked(clone!(weak, sender => move |_| {
            weak.upgrade().map(|h| h.add.on_add_clicked(&sender));
        }));

        let switch = &s.switch;
        let add_toggle = &s.add.toggle;
        let show_title = &s.show_title;
        let menu = &s.menu_button;
        s.back.connect_clicked(
            clone!(switch, add_toggle, show_title, sender, menu => move |back| {
                switch.show();
                add_toggle.show();
                back.hide();
                show_title.hide();
                menu.show();
                sender.send(Action::ShowShowsAnimated);
            }),
        );

        s.menu_button.set_menu_model(Some(&s.app_menu));
    }

    pub fn switch_to_back(&self, title: &str) {
        self.switch.hide();
        self.add.toggle.hide();
        self.back.show();
        self.set_show_title(title);
        self.show_title.show();
        self.menu_button.hide();
    }

    pub fn switch_to_normal(&self) {
        self.switch.show();
        self.add.toggle.show();
        self.back.hide();
        self.show_title.hide();
        self.menu_button.show();
    }

    pub fn set_show_title(&self, title: &str) {
        self.show_title.set_text(title)
    }

    pub fn show_update_notification(&self) {
        self.updater.show();
    }

    pub fn hide_update_notification(&self) {
        self.updater.hide();
    }

    pub fn open_menu(&self) {
        self.menu_button.clicked();
    }
}

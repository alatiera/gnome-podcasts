use gio;
use gtk;
use gtk::prelude::*;

use crossbeam_channel::Sender;
use failure::Error;
use rayon;
use url::Url;

use podcasts_data::{dbqueries, Source};

use app::Action;
use stacks::Content;
use utils::{itunes_to_rss, refresh};

use std::rc::Rc;

use i18n::i18n;

#[derive(Debug, Clone)]
// TODO: Factor out the hamburger menu
// TODO: Make a proper state machine for the headerbar states
pub struct Header {
    pub container: gtk::HeaderBar,
    switch: gtk::StackSwitcher,
    back: gtk::Button,
    show_title: gtk::Label,
    hamburger: gtk::MenuButton,
    updater: UpdateIndicator,
    add: AddPopover,
    dots: gtk::MenuButton,
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
        let mut url = self
            .entry
            .get_text()
            .ok_or_else(|| format_err!("GtkEntry blew up somehow."))?;

        if !(url.starts_with("https://") || url.starts_with("http://")) {
            url = format!("http://{}", url);
        };

        debug!("Url: {}", url);
        let url = if url.contains("itunes.com") || url.contains("apple.com") {
            info!("Detected itunes url.");
            let foo = itunes_to_rss(&url)?;
            info!("Resolved to {}", foo);
            foo
        } else {
            url.to_owned()
        };

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
        let mut url = self
            .entry
            .get_text()
            .ok_or_else(|| format_err!("GtkEntry blew up somehow."))?;
        debug!("Url: {}", url);

        if !(url.starts_with("https://") || url.starts_with("http://")) {
            url = format!("http://{}", url);
        };

        debug!("Url: {}", url);
        // TODO: refactor to avoid duplication
        match Url::parse(&url) {
            Ok(u) => {
                if !dbqueries::source_exists(u.as_str())? {
                    self.add.set_sensitive(true);
                    self.result.hide();
                    self.result.set_label("");
                } else {
                    self.add.set_sensitive(false);
                    self.result
                        .set_label(i18n("You are already subscribed to this Show").as_str());
                    self.result.show();
                }
                Ok(())
            }
            Err(err) => {
                self.add.set_sensitive(false);
                if !url.is_empty() {
                    self.result.set_label(i18n("Invalid url").as_str());
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
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/headerbar.ui");
        let menus = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/hamburger.ui");

        let header = builder.get_object("headerbar").unwrap();
        let switch = builder.get_object("switch").unwrap();
        let back = builder.get_object("back").unwrap();
        let show_title = builder.get_object("show_title").unwrap();

        // The hamburger menu
        let hamburger: gtk::MenuButton = builder.get_object("hamburger").unwrap();
        let app_menu: gio::MenuModel = menus.get_object("menu").unwrap();
        hamburger.set_menu_model(Some(&app_menu));

        // The 3 dots secondary menu
        let dots = builder.get_object("secondary_menu").unwrap();

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
            hamburger,
            updater,
            add,
            dots,
        }
    }
}

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

        s.back.connect_clicked(clone!(weak, sender => move |_| {
            weak.upgrade().map(|h| h.switch_to_normal());
            sender.send(Action::ShowShowsAnimated);
        }));
    }

    pub fn switch_to_back(&self, title: &str) {
        self.switch.hide();
        self.add.toggle.hide();
        self.back.show();
        self.set_show_title(title);
        self.show_title.show();
        self.hamburger.hide();
        self.dots.show();
    }

    pub fn switch_to_normal(&self) {
        self.switch.show();
        self.add.toggle.show();
        self.back.hide();
        self.show_title.hide();
        self.hamburger.show();
        self.dots.hide();
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
        self.hamburger.clicked();
    }

    pub fn set_secondary_menu(&self, pop: &gtk::PopoverMenu) {
        self.dots.set_popover(Some(pop));
    }
}

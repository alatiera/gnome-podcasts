// headerbar.rs
//
// Copyright 2017 Jordan Petridis <jpetridis@gnome.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gio;
use gtk;
use gtk::prelude::*;

use crossbeam_channel::Sender;
use failure::Error;
use rayon;
use url::Url;

use podcasts_data::{dbqueries, Source};

use crate::app::Action;
use crate::stacks::Content;
use crate::utils::{itunes_to_rss, refresh};

use std::rc::Rc;

use crate::i18n::i18n;

#[derive(Debug, Clone)]
// TODO: Make a proper state machine for the headerbar states
pub(crate) struct Header {
    pub(crate) container: gtk::HeaderBar,
    pub(crate) switch: gtk::StackSwitcher,
    back: gtk::Button,
    show_title: gtk::Label,
    hamburger: gtk::MenuButton,
    add: AddPopover,
    dots: gtk::MenuButton,
}

#[derive(Debug, Clone)]
struct AddPopover {
    container: gtk::Popover,
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
            url = format!("http://{}", url).into();
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
            url = format!("http://{}", url).into();
        };

        debug!("Url: {}", url);
        // TODO: refactor to avoid duplication
        match Url::parse(&url) {
            Ok(u) => {
                if !dbqueries::source_exists(u.as_str())? {
                    self.entry
                        .set_icon_from_icon_name(gtk::EntryIconPosition::Secondary, None);
                    self.add.set_sensitive(true);
                } else {
                    self.entry.set_icon_from_icon_name(
                        gtk::EntryIconPosition::Secondary,
                        "dialog-error-symbolic",
                    );
                    self.entry.set_icon_tooltip_text(
                        gtk::EntryIconPosition::Secondary,
                        i18n("You are already subscribed to this show").as_str(),
                    );
                    self.add.set_sensitive(false);
                }
                Ok(())
            }
            Err(err) => {
                self.add.set_sensitive(false);
                if !url.is_empty() {
                    self.entry.set_icon_from_icon_name(
                        gtk::EntryIconPosition::Secondary,
                        "dialog-error-symbolic",
                    );
                    self.entry.set_icon_tooltip_text(
                        gtk::EntryIconPosition::Secondary,
                        i18n("Invalid URL").as_str(),
                    );
                    error!("Error: {}", err);
                } else {
                    self.entry
                        .set_icon_from_icon_name(gtk::EntryIconPosition::Secondary, None);
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

        let add_toggle = builder.get_object("add_toggle").unwrap();
        let add_popover = builder.get_object("add_popover").unwrap();
        let new_url = builder.get_object("new_url").unwrap();
        let add_button = builder.get_object("add_button").unwrap();
        let add = AddPopover {
            container: add_popover,
            entry: new_url,
            toggle: add_toggle,
            add: add_button,
        };

        Header {
            container: header,
            switch,
            back,
            show_title,
            hamburger,
            add,
            dots,
        }
    }
}

// TODO: Make a proper state machine for the headerbar states
impl Header {
    pub(crate) fn new(content: &Content, sender: &Sender<Action>) -> Rc<Self> {
        let h = Rc::new(Header::default());
        Self::init(&h, content, &sender);
        h
    }

    pub(crate) fn init(s: &Rc<Self>, content: &Content, sender: &Sender<Action>) {
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
            sender.send(Action::ShowShowsAnimated).expect("Action channel blew up somehow");
        }));
    }

    pub(crate) fn switch_to_back(&self, title: &str) {
        self.switch.hide();
        self.add.toggle.hide();
        self.back.show();
        self.set_show_title(title);
        self.show_title.show();
        self.hamburger.hide();
        self.dots.show();
    }

    pub(crate) fn switch_to_normal(&self) {
        self.switch.show();
        self.add.toggle.show();
        self.back.hide();
        self.show_title.hide();
        self.hamburger.show();
        self.dots.hide();
    }

    pub(crate) fn set_show_title(&self, title: &str) {
        self.show_title.set_text(title)
    }

    pub(crate) fn open_menu(&self) {
        self.hamburger.clicked();
    }

    pub(crate) fn set_secondary_menu(&self, pop: &gtk::PopoverMenu) {
        self.dots.set_popover(Some(pop));
    }
}

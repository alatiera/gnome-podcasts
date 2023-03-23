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

use glib::clone;
use gtk::prelude::*;
use gtk::{gio, glib};

use anyhow::Result;
use glib::Sender;
use url::Url;

use podcasts_data::{dbqueries, Source};

use crate::app::Action;
use crate::stacks::Content;
use crate::utils::{itunes_to_rss, schedule_refresh, soundcloud_to_rss};

use std::rc::Rc;

use crate::i18n::i18n;

#[derive(Debug, Clone)]
// TODO: Make a proper state machine for the headerbar states
pub(crate) struct Header {
    pub(crate) container: adw::HeaderBar,
    pub(crate) switch: adw::ViewSwitcher,
    pub(crate) bottom_switcher: adw::ViewSwitcherBar,
    switch_squeezer: adw::Squeezer,
    back: gtk::Button,
    title_stack: gtk::Stack,
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

async fn add_podcast_from_url(url_input: String, sender: &Sender<Action>) -> Result<()> {
    let mut url = url_input;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        url = format!("http://{}", url);
    };

    debug!("Url: {}", url);
    let url = if url.contains("itunes.com") || url.contains("apple.com") {
        info!("Detected itunes url.");
        let itunes_url = itunes_to_rss(&url).await?;
        info!("Resolved to {}", itunes_url);
        itunes_url
    } else if url.contains("soundcloud.com") && !url.contains("feeds.soundcloud.com") {
        info!("Detected soundcloud url.");
        let soundcloud_url = soundcloud_to_rss(&Url::parse(&url)?).await?;
        info!("Resolved to {}", soundcloud_url);
        soundcloud_url.to_string()
    } else {
        url.to_owned()
    };

    rayon::spawn(clone!(@strong sender => move || {
        if let Ok(source) = Source::from_url(&url) {
            schedule_refresh(Some(vec![source]), sender.clone());
        } else {
            error!("Failed to convert, url: {}, to a source entry", url);
        }
    }));
    Ok(())
}

impl AddPopover {
    // FIXME: THIS ALSO SUCKS!
    fn on_add_clicked(&self, sender: &Sender<Action>) -> Result<()> {
        let url = self.entry.text();

        tokio::spawn(clone!(@strong sender => async move {
            add_podcast_from_url(url.to_string(), &sender).await;
        }));

        self.container.hide();
        Ok(())
    }

    // FIXME: THIS SUCKS! REFACTOR ME.
    fn on_entry_changed(&self) -> Result<()> {
        let mut url = self.entry.text();
        let is_input_url_empty = url.is_empty();
        debug!("Url: {}", url);

        if !(url.starts_with("https://") || url.starts_with("http://")) {
            url = format!("http://{}", url).into();
        };

        debug!("Url: {}", url);
        match Url::parse(&url) {
            Ok(u) => {
                if !dbqueries::source_exists(u.as_str())? {
                    self.style_neutral(true);
                } else {
                    self.style_error("You are already subscribed to this show");
                }
                Ok(())
            }
            Err(err) => {
                if !is_input_url_empty {
                    self.style_error("Invalid URL");
                    error!("Error: {}", err);
                } else {
                    self.style_neutral(false);
                }
                Ok(())
            }
        }
    }

    fn style_error(&self, icon_tooltip: &str) {
        self.style(
            true,
            false,
            Some("dialog-error-symbolic"),
            Some(icon_tooltip),
        );
    }

    fn style_neutral(&self, sensitive: bool) {
        self.style(false, sensitive, None, None);
    }

    fn style(
        &self,
        error: bool,
        sensitive: bool,
        icon_name: Option<&str>,
        icon_tooltip: Option<&str>,
    ) {
        let entry = &self.entry;
        entry.set_secondary_icon_name(icon_name);
        if let Some(icon_tooltip_text) = icon_tooltip {
            entry.set_secondary_icon_tooltip_text(Some(i18n(icon_tooltip_text).as_str()));
        }
        self.add.set_sensitive(sensitive);

        if error {
            entry.add_css_class("error");
        } else {
            entry.remove_css_class("error");
        }
    }
}

impl Default for Header {
    fn default() -> Header {
        let builder = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/headerbar.ui");
        let menus = gtk::Builder::from_resource("/org/gnome/Podcasts/gtk/hamburger.ui");

        let header = builder.object("headerbar").unwrap();
        let switch: adw::ViewSwitcher = builder.object("switch").unwrap();
        let back = builder.object("back").unwrap();
        let title_stack = builder.object("title_stack").unwrap();
        let switch_squeezer: adw::Squeezer = builder.object("switch_squeezer").unwrap();
        let show_title = builder.object("show_title").unwrap();

        // The hamburger menu
        let hamburger: gtk::MenuButton = builder.object("hamburger").unwrap();
        let app_menu: gio::MenuModel = menus.object("menu").unwrap();
        hamburger.set_menu_model(Some(&app_menu));

        // The 3 dots secondary menu
        let dots = builder.object("secondary_menu").unwrap();

        let add_toggle = builder.object("add_toggle").unwrap();
        let add_popover = builder.object("add_popover").unwrap();
        let new_url = builder.object("new_url").unwrap();
        let add_button = builder.object("add_button").unwrap();
        let add = AddPopover {
            container: add_popover,
            entry: new_url,
            toggle: add_toggle,
            add: add_button,
        };

        // View switcher bar that goes at the bottom of the window
        let switcher = adw::ViewSwitcherBar::new();
        switcher.set_reveal(false);

        Header {
            container: header,
            switch,
            back,
            title_stack,
            switch_squeezer,
            bottom_switcher: switcher,
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
        Self::init(&h, content, sender);
        h
    }

    pub(crate) fn init(s: &Rc<Self>, content: &Content, sender: &Sender<Action>) {
        s.bottom_switcher.set_stack(Some(&content.get_stack()));
        s.switch.set_stack(Some(&content.get_stack()));

        s.add.entry.connect_changed(clone!(@weak s => move |_| {
            s.add.on_entry_changed()
            .map_err(|err| error!("Error: {}", err))
            .ok();
        }));

        s.add
            .add
            .connect_clicked(clone!(@weak s, @strong sender => move |_| {
                s.add.on_add_clicked(&sender).unwrap();
            }));

        s.add
            .entry
            .connect_activate(clone!(@weak s, @strong sender => move |_| {
                if s.add.add.get_sensitive() {
                        s.add.on_add_clicked(&sender).unwrap();
                    }
            }));

        s.back
            .connect_clicked(clone!(@weak s, @strong sender => move |_| {
                s.switch_to_normal();
                send!(sender, Action::ShowShowsAnimated);
            }));

        s.switch_squeezer
            .connect_visible_child_notify(clone!(@weak s => move |_| {
                s.update_bottom_switcher();
            }));
        s.update_bottom_switcher();
    }

    pub(crate) fn switch_to_back(&self, title: &str) {
        self.add.toggle.hide();
        self.back.show();
        self.set_show_title(title);
        self.title_stack.set_visible_child(&self.show_title);
        self.bottom_switcher.set_reveal(false);
        self.hamburger.hide();
        self.dots.show();
    }

    pub(crate) fn switch_to_normal(&self) {
        self.add.toggle.show();
        self.back.hide();
        self.title_stack.set_visible_child(&self.switch_squeezer);
        self.hamburger.show();
        self.update_bottom_switcher();
        self.dots.hide();
    }

    pub(crate) fn set_show_title(&self, title: &str) {
        self.show_title.set_text(title)
    }

    pub(crate) fn open_menu(&self) {
        self.hamburger.popup();
    }

    pub(crate) fn set_secondary_menu(&self, menu: &gio::MenuModel) {
        self.dots.set_menu_model(Some(menu))
    }

    pub(crate) fn reveal_bottom_switcher(&self, value: bool) {
        self.bottom_switcher.set_reveal(value);
    }

    pub(crate) fn update_bottom_switcher(&self) {
        if let Some(child) = self.switch_squeezer.visible_child() {
            // only show the bottom switcher if we are on the current page
            // and have no title menu customization (e.g.: ShowWidget)
            let reveal = (child != self.switch) && self.hamburger.is_visible();
            self.bottom_switcher.set_reveal(reveal);
        }
    }
}

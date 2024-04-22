// chapters_page.rs
//
// Copyright 2025 nee <nee-git@patchouli.garden>
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

use adw::subclass::prelude::*;
use anyhow::Result;
use async_channel::Sender;
use glib::subclass::InitializingObject;
use gtk::CompositeTemplate;
use gtk::glib;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};

use crate::app::Action;
use crate::chapter_parser::Chapter;
use podcasts_data::EpisodeId;
use podcasts_data::dbqueries;

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/chapters_page.ui")]
pub struct ChaptersPagePriv {
    #[template_child]
    title: TemplateChild<gtk::Label>,
    #[template_child]
    listbox: TemplateChild<gtk::ListBox>,
    episode_id: Cell<Option<EpisodeId>>,
    sender: RefCell<Option<Sender<Action>>>,
}

impl ChaptersPagePriv {
    fn init(&self, id: EpisodeId) -> Result<()> {
        let ep = dbqueries::get_episode_widget_from_id(id)?;
        let show = dbqueries::get_podcast_from_id(ep.show_id())?;
        self.title
            .set_text(&format!("{} - {}", show.title(), ep.title()));
        Ok(())
    }

    fn fill_chapters_list(&self, chapters: Vec<Chapter>) {
        for c in chapters.into_iter() {
            let item = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            item.set_margin_top(15);
            item.set_margin_bottom(15);
            item.set_margin_start(15);
            item.set_margin_end(15);

            let s = c.start.num_seconds();
            let duration = {
                let seconds = s % 60;
                let minutes = (s / 60) % 60;
                let hours = (s / 60) / 60;
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            };
            let time = gtk::Label::new(Some(&duration));
            time.set_margin_end(15);
            item.append(&time);

            let title = gtk::Label::new(Some(&c.title));
            title.set_wrap(true);
            title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            item.append(&title);

            let row = gtk::ListBoxRow::new();
            row.set_action_name(Some("jump-to-second"));
            row.set_action_target_value(Some(&(s as i32).to_variant()));
            row.set_child(Some(&item));
            if !c.description.is_empty() {
                row.set_tooltip_text(Some(&c.description));
            }
            self.listbox.append(&row);
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ChaptersPagePriv {
    const NAME: &'static str = "PdChaptersPage";
    type Type = ChaptersPage;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.install_action(
            "jump-to-second",
            Some(glib::VariantTy::INT32),
            move |this, _, value| {
                if let Some(second) = value.and_then(|v| v.get::<i32>()) {
                    this.jump_to_second(second);
                }
            },
        );
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for ChaptersPagePriv {}
impl ObjectImpl for ChaptersPagePriv {}
impl NavigationPageImpl for ChaptersPagePriv {}

glib::wrapper! {
    pub struct ChaptersPage(ObjectSubclass<ChaptersPagePriv>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ChaptersPage {
    pub(crate) fn new(sender: &Sender<Action>, id: EpisodeId, chapters: Vec<Chapter>) -> Self {
        let this: Self = glib::Object::new();
        this.imp().fill_chapters_list(chapters);
        let _ = this.imp().init(id);
        this.imp().episode_id.set(Some(id));
        this.imp().sender.replace(Some(sender.clone()));

        this
    }

    fn jump_to_second(&self, second: i32) {
        let sender = self.imp().sender.borrow();
        if let (Some(id), Some(sender)) = (self.imp().episode_id.get(), sender.clone()) {
            send_blocking!(sender, Action::InitEpisodeAt(id, second));
        }
    }
}

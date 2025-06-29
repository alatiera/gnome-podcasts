// chapters.rs
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

use adw::prelude::*;
use adw::subclass::prelude::*;
use anyhow::Result;
use async_channel::Sender;
use glib::Properties;
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::{Cell, RefCell};

use crate::app::Action;
use crate::chapter_parser::Chapter;
use crate::i18n::i18n;
use podcasts_data::EpisodeId;
use podcasts_data::dbqueries;

#[derive(Debug, CompositeTemplate, Default, Properties)]
#[template(resource = "/org/gnome/Podcasts/gtk/chapters.ui")]
#[properties(wrapper_type = Chapters)]
pub struct ChaptersPriv {
    #[template_child]
    episode: TemplateChild<gtk::Label>,
    #[template_child]
    show: TemplateChild<gtk::Label>,
    #[template_child]
    listbox: TemplateChild<gtk::ListBox>,

    chapters: RefCell<Vec<Chapter>>,
    episode_id: Cell<Option<EpisodeId>>,
    sender: RefCell<Option<Sender<Action>>>,
    active_chapter_icon: RefCell<Option<gtk::Image>>,

    #[property(get, set)]
    progress: Cell<f64>,
    #[property(get, set)]
    active_chapter_index: Cell<i32>,
}

impl ChaptersPriv {
    fn set_labels(&self, id: EpisodeId) -> Result<()> {
        let ep = dbqueries::get_episode_widget_from_id(id)?;
        self.episode.set_text(ep.title());

        let show = dbqueries::get_podcast_from_id(ep.show_id())?;
        self.show.set_text(show.title());
        Ok(())
    }

    fn fill_chapters_list(&self, chapters: &[Chapter]) {
        self.listbox.remove_all();
        for c in chapters.iter() {
            let s = c.start.num_seconds();
            let duration = {
                let seconds = s % 60;
                let minutes = (s / 60) % 60;
                let hours = (s / 60) / 60;
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            };

            let row = adw::ActionRow::new();
            row.set_action_name(Some("jump-to-second"));
            row.set_action_target_value(Some(&(s as i32).to_variant()));
            row.set_activatable(true);
            row.set_title(&duration);
            row.set_subtitle(&c.title);
            row.add_css_class("property");
            if !c.description.is_empty() {
                row.set_tooltip_text(Some(&c.description));
            }
            self.listbox.append(&row);
        }
    }

    fn update_active_chapter(&self) {
        let seconds = self.progress.get();
        let old_index = self.active_chapter_index.get();
        let mut new_index = 0;
        for c in self.chapters.borrow().iter() {
            if c.start.num_seconds() as f64 > seconds {
                break;
            }
            new_index += 1;
        }
        new_index = (new_index - 1).max(0);

        if old_index != new_index {
            self.active_chapter_index.set(new_index);
            self.active_chapter_changed(old_index);
        }
    }

    fn active_chapter_changed(&self, old_index: i32) {
        println!("CHAP {}", self.active_chapter_index.get());
        if let Some(icon) = self.active_chapter_icon.borrow().as_ref() {
            if let Some(row) = self
                .listbox
                .row_at_index(old_index)
                .and_then(|r| r.downcast::<adw::ActionRow>().ok())
            {
                row.remove(icon);
            }
        }
        if let Some(row) = self
            .listbox
            .row_at_index(self.active_chapter_index.get())
            .and_then(|r| r.downcast::<adw::ActionRow>().ok())
        {
            let icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
            icon.set_pixel_size(12);
            icon.set_tooltip_text(Some(&i18n("Currently playing chapter")));
            row.add_suffix(&icon);
            self.active_chapter_icon.replace(Some(icon));
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ChaptersPriv {
    const NAME: &'static str = "PdChapters";
    type Type = Chapters;
    type ParentType = adw::Bin;

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

#[glib::derived_properties]
impl ObjectImpl for ChaptersPriv {}
impl WidgetImpl for ChaptersPriv {}
impl BinImpl for ChaptersPriv {}

glib::wrapper! {
    pub struct Chapters(ObjectSubclass<ChaptersPriv>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Chapters {
    pub(crate) fn new(
        sender: &Sender<Action>,
        slider: &gtk::Scale,
        ep: EpisodeId,
        chapters: Vec<Chapter>,
    ) -> Self {
        let this: Self = glib::Object::new();
        this.init(sender, slider);
        this.set_chapters(ep, chapters);
        this
    }

    pub(crate) fn new_page(
        sender: &Sender<Action>,
        slider: &gtk::Scale,
        ep: EpisodeId,
        chapters: Vec<Chapter>,
    ) -> adw::NavigationPage {
        let widget = Self::new(sender, slider, ep, chapters);
        let view = adw::ToolbarView::builder().content(&widget).build();
        view.add_top_bar(&adw::HeaderBar::new());
        adw::NavigationPage::with_tag(&view, &i18n("Chapters"), "chapters")
    }

    pub(crate) fn init(&self, sender: &Sender<Action>, slider: &gtk::Scale) {
        let imp = self.imp();
        imp.sender.replace(Some(sender.clone()));

        slider.connect_value_changed(clone!(
            #[weak]
            imp,
            move |slider| {
                let seconds = slider.value();
                imp.progress.set(seconds);
                imp.update_active_chapter();
            }
        ));
        imp.progress.set(slider.value());
        imp.update_active_chapter();
    }

    fn jump_to_second(&self, second: i32) {
        let sender = self.imp().sender.borrow();
        if let (Some(id), Some(sender)) = (self.imp().episode_id.get(), sender.clone()) {
            send_blocking!(sender, Action::InitEpisodeAt(id, second));
        }
    }

    pub(crate) fn set_chapters(&self, ep: EpisodeId, chapters: Vec<Chapter>) {
        let imp = self.imp();
        imp.episode_id.set(Some(ep));
        let _ = imp.set_labels(ep);
        imp.fill_chapters_list(&chapters);
        imp.chapters.replace(chapters);
        imp.update_active_chapter();
    }
}

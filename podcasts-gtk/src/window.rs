// window.rs
//
// Copyright 2019 Jordan Petridis <jpetridis@gnome.org>
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
use glib::clone;
use gst::ClockTime;
use gtk::CompositeTemplate;
use gtk::{gio, glib};
use std::cell::{Cell, OnceCell, RefCell};
use std::cell::{Ref, RefMut};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

use crate::app::{Action, PdApplication};
use crate::config::APP_ID;
use crate::feed_manager::FEED_MANAGER;
use crate::headerbar::Header;
use crate::settings::{self, WindowGeometry};
use crate::utils;
use crate::widgets::about_dialog;
use crate::widgets::player;
use crate::widgets::player::{PlayerExt, PlayerWidget, SeekDirection};
use crate::widgets::{Content, DiscoveryPage, EpisodeDescription, ShowWidget};
use podcasts_data::dbqueries;

#[derive(Debug, CompositeTemplate, glib::Properties)]
#[template(resource = "/org/gnome/Podcasts/gtk/window.ui")]
#[properties(wrapper_type = MainWindow)]
pub struct MainWindowPriv {
    pub(crate) content: OnceCell<Rc<Content>>,
    pub(crate) headerbar: OnceCell<Header>,
    pub(crate) player: OnceCell<player::PlayerWrapper>,
    pub(crate) updating_timeout: RefCell<Option<glib::source::SourceId>>,
    pub(crate) settings: gio::Settings,
    pub(crate) bottom_switcher: adw::ViewSwitcherBar,
    pub(crate) show_widget: RefCell<Option<ShowWidget>>,

    pub(crate) sender: OnceCell<Sender<Action>>,

    #[template_child]
    pub(crate) toolbar_view: TemplateChild<adw::ToolbarView>,
    #[template_child]
    pub(crate) player_toolbar_view: TemplateChild<adw::ToolbarView>,
    #[template_child]
    pub(crate) toast_overlay: TemplateChild<adw::ToastOverlay>,
    #[template_child]
    pub(crate) navigation_view: TemplateChild<adw::NavigationView>,
    #[template_child]
    pub(crate) header_breakpoint: TemplateChild<adw::Breakpoint>,
    #[template_child]
    pub(crate) player_breakpoint: TemplateChild<adw::Breakpoint>,
    #[template_child]
    pub(crate) show_page: TemplateChild<adw::NavigationPage>,

    #[property(set, get)]
    pub(crate) updating: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for MainWindowPriv {
    const NAME: &'static str = "PdMainWindow";
    type Type = MainWindow;
    type ParentType = adw::ApplicationWindow;

    fn new() -> Self {
        let settings = gio::Settings::new(APP_ID);

        Self {
            headerbar: OnceCell::new(),
            content: OnceCell::new(),
            player: OnceCell::new(),
            navigation_view: TemplateChild::default(),
            toast_overlay: TemplateChild::default(),
            toolbar_view: TemplateChild::default(),
            player_toolbar_view: TemplateChild::default(),
            header_breakpoint: TemplateChild::default(),
            player_breakpoint: TemplateChild::default(),
            show_page: TemplateChild::default(),
            bottom_switcher: adw::ViewSwitcherBar::new(),
            updating: Cell::new(false),
            updating_timeout: RefCell::new(None),
            sender: OnceCell::new(),
            show_widget: RefCell::new(None),
            settings,
        }
    }

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.install_action("win.refresh", None, move |win, _, _| {
            let sender = win.sender();
            FEED_MANAGER.schedule_full_refresh(sender);
        });
        klass.install_action_async("win.import", None, |win, _, _| async move {
            let sender = win.sender();
            utils::on_import_clicked(win.upcast_ref(), sender).await;
        });
        klass.install_action_async("win.export", None, |win, _, _| async move {
            let sender = win.sender();
            utils::on_export_clicked(win.upcast_ref(), sender).await;
        });
        klass.install_action("win.about", None, move |win, _, _| {
            about_dialog(win.upcast_ref());
        });
        klass.install_action("win.toggle-pause", None, move |win, _, _| {
            win.player_mut().toggle_pause();
        });
        klass.install_action("win.seek-forwards", None, move |win, _, _| {
            win.player()
                .seek(ClockTime::from_seconds(10), SeekDirection::Forward);
        });
        klass.install_action("win.seek-backwards", None, move |win, _, _| {
            win.player()
                .seek(ClockTime::from_seconds(5), SeekDirection::Backwards);
        });
        klass.install_action("win.go-to-home", None, move |win, _, _| {
            win.pop_to_home();
            win.content().go_to_home();
        });
        klass.install_action("win.go-to-shows", None, move |win, _, _| {
            win.pop_to_home();
            win.content().go_to_shows();
        });
        klass.install_action("win.go-to-discovery", None, move |win, _, _| {
            win.go_to_discovery();
        });
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for MainWindowPriv {
    fn constructed(&self) {
        let window = self.obj();
        self.parent_constructed();

        if APP_ID.ends_with("Devel") {
            window.add_css_class("devel");
        }

        // Retrieve the previous window position and size.
        WindowGeometry::from_settings(&self.settings).apply(window.upcast_ref());
    }
}

impl WidgetImpl for MainWindowPriv {}
impl WindowImpl for MainWindowPriv {
    // Save window state on delete event
    fn close_request(&self) -> glib::Propagation {
        let obj = self.obj();
        info!("Saving window position");

        WindowGeometry::from_window(obj.upcast_ref()).write(&self.settings);

        self.parent_close_request()
    }
}
impl ApplicationWindowImpl for MainWindowPriv {}
impl AdwApplicationWindowImpl for MainWindowPriv {}

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<MainWindowPriv>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

impl MainWindow {
    pub(crate) fn new(app: &PdApplication, sender: &Sender<Action>) -> Self {
        let window: Self = glib::Object::builder().property("application", app).build();
        let imp = window.imp();
        let content = Content::new(sender.clone());
        let header = Header::new(&content);

        imp.sender.set(sender.clone()).unwrap();

        imp.toolbar_view.add_top_bar(&header.container);
        imp.toolbar_view.set_content(Some(content.overlay()));
        imp.toolbar_view.add_bottom_bar(&imp.bottom_switcher);

        imp.bottom_switcher.set_stack(Some(content.stack()));

        let player = player::PlayerWrapper::new(sender);
        imp.player_toolbar_view
            .add_bottom_bar(&player.borrow().container);

        // Setup breakpoints
        imp.header_breakpoint.add_setter(
            &header.container,
            "title-widget",
            &gtk::Widget::NONE.to_value(),
        );
        imp.header_breakpoint
            .add_setter(&imp.bottom_switcher, "reveal", &true.to_value());
        let p = player.deref();
        imp.player_breakpoint
            .connect_apply(clone!(@weak p => move |_| {
                p.borrow().set_small(false);
            }));
        imp.player_breakpoint
            .connect_unapply(clone!(@weak p => move |_| {
                p.borrow().set_small(true);
            }));
        let breakpoint = imp.player_breakpoint.get();
        let is_small = !window.current_breakpoint().is_some_and(|b| b == breakpoint);
        p.borrow().set_small(is_small);

        // Update the feeds right after the Window is initialized.
        if imp.settings.boolean("refresh-on-startup") {
            info!("Refresh on startup.");
            FEED_MANAGER.schedule_full_refresh(sender);
        }

        let refresh_interval = settings::get_refresh_interval(&imp.settings).num_seconds() as u32;
        info!("Auto-refresh every {:?} seconds.", refresh_interval);

        glib::timeout_add_seconds_local(
            refresh_interval,
            clone!(@strong sender => move || {
                FEED_MANAGER.schedule_full_refresh(&sender);
                glib::ControlFlow::Continue
            }),
        );

        imp.headerbar.set(header).unwrap();
        imp.content.set(content).unwrap();
        imp.player.set(player).unwrap();

        window
    }

    pub fn push_page<P: IsA<adw::NavigationPage>>(&self, page: &P) {
        self.imp().navigation_view.push(page);
    }

    fn pop_to_show_widget(&self) {
        self.imp().navigation_view.pop_to_tag("show");
    }

    fn pop_to_home(&self) {
        self.imp().navigation_view.pop_to_tag("home");
    }

    pub(crate) fn init_episode(&self, id: i32, second: Option<i32>, stream: bool) -> Result<()> {
        self.imp()
            .player
            .get()
            .unwrap()
            .borrow_mut()
            .initialize_episode(id, stream, second)
    }

    pub(crate) fn add_toast(&self, toast: adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }

    pub(crate) fn set_updating_timeout(&self, timeout: Option<glib::source::SourceId>) {
        if let Some(old_timeout) = self.imp().updating_timeout.replace(timeout) {
            old_timeout.remove();
        }
    }

    pub(crate) fn progress_bar(&self) -> &gtk::ProgressBar {
        self.content().progress_bar()
    }

    pub(crate) fn player(&self) -> Ref<'_, PlayerWidget> {
        self.imp().player.get().unwrap().borrow()
    }

    pub(crate) fn player_mut(&self) -> RefMut<'_, PlayerWidget> {
        self.imp().player.get().unwrap().borrow_mut()
    }

    pub(crate) fn content(&self) -> &Content {
        self.imp().content.get().unwrap()
    }

    pub(crate) fn headerbar(&self) -> &Header {
        self.imp().headerbar.get().unwrap()
    }

    pub(crate) fn bottom_switcher(&self) -> &adw::ViewSwitcherBar {
        &self.imp().bottom_switcher
    }

    pub(crate) fn sender(&self) -> &Sender<Action> {
        self.imp().sender.get().unwrap()
    }

    pub(crate) fn go_to_discovery(&self) {
        let widget = DiscoveryPage::new(self.sender());
        self.push_page(&widget);
    }

    pub(crate) fn replace_show_widget(&self, widget: Option<ShowWidget>, title: &str) {
        let imp = self.imp();
        let is_current_page = imp
            .navigation_view
            .visible_page()
            .is_some_and(|p| p == *imp.show_page);
        imp.show_page.set_child(widget.as_ref());
        if widget.is_some() {
            imp.show_page.set_title(title);
        } else if is_current_page {
            imp.navigation_view.pop();
        }
        imp.show_widget.set(widget);
    }

    pub(crate) fn update_show_widget(&self, show_id: i32) -> Result<()> {
        let imp = self.imp();
        let same = imp.show_widget.borrow().as_ref().and_then(|s| s.show_id()) == Some(show_id);
        if same {
            let pd = dbqueries::get_podcast_from_id(show_id)?;
            let widget = ShowWidget::new(Arc::new(pd), self.sender());
            self.replace_show_widget(Some(widget), &imp.show_page.title());
        }
        Ok(())
    }

    pub(crate) fn go_to_show_widget(&self) {
        let imp = self.imp();
        let is_current_page = imp
            .navigation_view
            .visible_page()
            .is_some_and(|p| p == *imp.show_page);
        if !is_current_page {
            self.pop_to_show_widget();
            imp.navigation_view.push(&*imp.show_page);
        }
    }

    pub(crate) fn pop_show_widget(&self) {
        let imp = self.imp();
        let is_current_page = imp
            .navigation_view
            .visible_page()
            .is_some_and(|p| p == *imp.show_page);
        if is_current_page {
            imp.navigation_view.pop();
        }
    }

    pub(crate) fn pop_episode_description(&self) {
        let imp = self.imp();
        let is_current_page = imp
            .navigation_view
            .visible_page()
            .is_some_and(|p| p.downcast::<EpisodeDescription>().is_ok());

        if is_current_page {
            imp.navigation_view.pop();
        }
    }
}

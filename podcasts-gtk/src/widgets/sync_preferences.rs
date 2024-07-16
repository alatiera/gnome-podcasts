// sync_preferences.rs
//
// Copyright 2023-2024 nee <nee-git@patchouli.garden>
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
use anyhow::{anyhow, Result};
use async_channel::Sender;
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;

use crate::app::Action;
use crate::i18n::i18n;
use podcasts_data::feed_manager::FEED_MANAGER;
use podcasts_data::nextcloud_sync::{self, SyncPreferences};

pub enum WidgetAction {
    GotSettings(Result<(podcasts_data::sync::Settings, String)>),
    LoadingMessage(String),
    LogoutDone,
}

#[derive(Debug, CompositeTemplate, Default)]
#[template(resource = "/org/gnome/Podcasts/gtk/sync_preferences.ui")]
pub struct SyncPreferencesPriv {
    #[template_child]
    server: TemplateChild<adw::EntryRow>,
    #[template_child]
    username: TemplateChild<adw::EntryRow>,
    #[template_child]
    password: TemplateChild<adw::PasswordEntryRow>,

    #[template_child]
    server_label: TemplateChild<adw::EntryRow>,
    #[template_child]
    user_label: TemplateChild<adw::EntryRow>,

    #[template_child]
    login_browser: TemplateChild<gtk::Button>,
    #[template_child]
    login_password: TemplateChild<gtk::Button>,
    #[template_child]
    logout: TemplateChild<gtk::Button>,

    #[template_child]
    method_browser: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    method_password: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    connection_info: TemplateChild<adw::PreferencesGroup>,
    #[template_child]
    login_group: TemplateChild<adw::PreferencesGroup>,

    #[template_child]
    last_sync: TemplateChild<gtk::Label>,
    #[template_child]
    active: TemplateChild<adw::SwitchRow>,
    #[template_child]
    sync_now: TemplateChild<gtk::Button>,

    #[template_child]
    loading_info: TemplateChild<gtk::Box>,
    #[template_child]
    loading_label: TemplateChild<gtk::Label>,
    #[template_child]
    loading_spinner: TemplateChild<gtk::Spinner>,
}

impl SyncPreferencesPriv {
    fn init(&self, sender: Sender<Action>) {
        let (widget_sender, receiver) = async_channel::bounded(1);
        crate::MAINCONTEXT.spawn_local(clone!(
            #[weak(rename_to = this)]
            self,
            async move {
                while let Ok(action) = receiver.recv().await {
                    this.do_action(action);
                }
            }
        ));

        // inital load
        crate::RUNTIME.spawn(Self::fetch_settings(widget_sender.clone()));

        self.method_browser.connect_toggled(clone!(
            #[weak(rename_to = this)]
            self,
            move |b| {
                if b.is_active() {
                    this.username.set_visible(false);
                    this.password.set_visible(false);
                    this.login_password.set_visible(false);
                    this.login_browser.set_visible(true);
                }
            }
        ));

        self.method_password.connect_toggled(clone!(
            #[weak(rename_to = this)]
            self,
            move |b| {
                if b.is_active() {
                    this.username.set_visible(true);
                    this.password.set_visible(true);
                    this.login_password.set_visible(true);
                    this.login_browser.set_visible(false);
                }
            }
        ));

        self.login_password.connect_clicked(clone!(
            #[strong]
            sender,
            #[strong]
            widget_sender,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let server = this.server.text();
                let user = this.username.text();
                let password = this.password.text();

                this.enter_loading_state(&i18n("Logging in..."));

                crate::RUNTIME.spawn(clone!(
                    #[strong]
                    sender,
                    #[strong]
                    widget_sender,
                    #[strong]
                    server,
                    #[strong]
                    user,
                    #[strong]
                    password,
                    async move {
                        let refresh_sender = sender.clone();
                        let err_sender = widget_sender.clone();
                        if let Err(e) = async move {
                            let app_password =
                                nextcloud_sync::retrive_app_password(&server, &user, &password)
                                    .await?;
                            Self::do_first_sync(
                                refresh_sender,
                                widget_sender.clone(),
                                &server,
                                &user,
                                &app_password,
                            )
                            .await
                        }
                        .await
                        {
                            send!(
                                sender,
                                Action::ErrorNotification(format!("Login error: {e}"))
                            );
                            Self::fetch_settings(err_sender).await;
                        }
                    }
                ));
            }
        ));

        self.login_browser.connect_clicked(clone!(
            #[strong]
            sender,
            #[strong]
            widget_sender,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let server = this.server.text();
                let sender = sender.clone();
                let widget_sender = widget_sender.clone();

                this.enter_loading_state(&i18n("Waiting for browser login..."));

                crate::RUNTIME.spawn(async move {
                    let refresh_sender = sender.clone();
                    let err_sender = widget_sender.clone();
                    if let Err(e) = async move {
                        let (server, user, app_password) =
                            nextcloud_sync::launch_browser_login_flow_v2(&server, |url| {
                                let launch_context: Option<&gtk::gio::AppLaunchContext> = None;
                                Ok(gtk::gio::AppInfo::launch_default_for_uri(
                                    url,
                                    launch_context,
                                )?)
                            })
                            .await?;
                        Self::do_first_sync(
                            refresh_sender,
                            widget_sender.clone(),
                            &server,
                            &user,
                            &app_password,
                        )
                        .await
                    }
                    .await
                    {
                        send!(
                            sender,
                            Action::ErrorNotification(format!("Login error: {e}"))
                        );
                        Self::fetch_settings(err_sender).await;
                    }
                });
            }
        ));

        self.logout.connect_clicked(clone!(
            #[strong]
            sender,
            #[strong]
            widget_sender,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let sender = sender.clone();
                let widget_sender = widget_sender.clone();

                this.enter_loading_state(&i18n("Logging out..."));

                crate::RUNTIME.spawn(async move {
                    let err_sender = widget_sender.clone();
                    if let Err(e) = async move {
                        let (settings, password) = podcasts_data::sync::Settings::fetch().await?;
                        nextcloud_sync::logout(&settings.server, &settings.user, &password).await;
                        podcasts_data::sync::Settings::remove().await?;
                        send!(widget_sender, WidgetAction::LogoutDone);
                        anyhow::Ok(())
                    }
                    .await
                    {
                        send!(
                            sender,
                            Action::ErrorNotification(format!("Logout error: {e}"))
                        );
                        Self::fetch_settings(err_sender).await;
                    }
                });
            }
        ));

        self.login_browser.connect_clicked(clone!(
            #[strong]
            sender,
            #[strong]
            widget_sender,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let server = this.server.text();
                let sender = sender.clone();
                let widget_sender = widget_sender.clone();

                this.enter_loading_state(&i18n("Waiting for browser login..."));

                crate::RUNTIME.spawn(async move {
                    let refresh_sender = sender.clone();
                    let err_sender = widget_sender.clone();
                    if let Err(e) = async move {
                        let (server, user, app_password) =
                            nextcloud_sync::launch_browser_login_flow_v2(&server, |url| {
                                let launch_context: Option<&gtk::gio::AppLaunchContext> = None;
                                Ok(gtk::gio::AppInfo::launch_default_for_uri(
                                    url,
                                    launch_context,
                                )?)
                            })
                            .await?;
                        Self::do_first_sync(
                            refresh_sender,
                            widget_sender.clone(),
                            &server,
                            &user,
                            &app_password,
                        )
                        .await
                    }
                    .await
                    {
                        send!(
                            sender,
                            Action::ErrorNotification(format!("Login error: {e}"))
                        );
                        Self::fetch_settings(err_sender).await;
                    }
                });
            }
        ));

        self.logout.connect_clicked(clone!(
            #[strong]
            sender,
            #[strong]
            widget_sender,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let sender = sender.clone();
                let widget_sender = widget_sender.clone();

                this.enter_loading_state(&i18n("Logging out..."));

                crate::RUNTIME.spawn(async move {
                    let err_sender = widget_sender.clone();
                    if let Err(e) = async move {
                        let (settings, password) = podcasts_data::sync::Settings::fetch().await?;
                        nextcloud_sync::logout(&settings.server, &settings.user, &password).await;
                        podcasts_data::sync::Settings::remove().await?;
                        if let Err(e) = widget_sender.send(WidgetAction::LogoutDone).await {
                            error!("failed to send {e}");
                        }
                        anyhow::Ok(())
                    }
                    .await
                    {
                        send!(
                            sender,
                            Action::ErrorNotification(format!("Logout error: {e}"))
                        );
                        Self::fetch_settings(err_sender).await;
                    }
                });
            }
        ));

        self.sync_now.connect_clicked(clone!(
            #[strong]
            sender,
            #[strong]
            widget_sender,
            #[weak(rename_to = this)]
            self,
            move |_| {
                let sender = sender.clone();
                let widget_sender = widget_sender.clone();
                this.enter_loading_state(&i18n("Synchronizing now..."));
                crate::RUNTIME.spawn(async move {
                    send!(
                        widget_sender,
                        WidgetAction::LoadingMessage(i18n("Refreshing feeds..."))
                    );
                    FEED_MANAGER.full_refresh().await;
                    let result = nextcloud_sync::sync(SyncPolicy::IgnoreMissingEpisodes).await;
                    match result {
                        Err(e) => send!(
                            sender,
                            Action::ErrorNotification(format!("Sync error: {:#?}", e))
                        ),
                        _ => info!("SYNC SUCCESS"),
                    }
                    Self::fetch_settings(widget_sender).await;
                });
            }
        ));

        self.active.connect_active_notify(move |b| {
            let active = b.is_active();
            gio::spawn_blocking(move || {
                podcasts_data::sync::Settings::set_active(active)?;
                anyhow::Ok(())
            });
        });
    }

    async fn do_first_sync(
        app_sender: Sender<Action>,
        widget_sender: Sender<WidgetAction>,
        server: &str,
        user: &str,
        app_password: &str,
    ) -> Result<()> {
        podcasts_data::sync::Settings::store(server, user, app_password).await?;
        send!(
            widget_sender,
            WidgetAction::LoadingMessage(i18n("Refreshing feeds..."))
        );
        FEED_MANAGER.full_refresh().await;
        send!(
            widget_sender,
            WidgetAction::LoadingMessage(i18n("Running first sync..."))
        );
        // IgnoreMissingEpisodes, because we just did a full refresh.
        // Also episodes might be missing if a feed 404s.
        // And finishing the login/first sync is more important for UX.
        nextcloud_sync::sync(SyncPolicy::IgnoreMissingEpisodes).await?;
        Self::fetch_settings(widget_sender).await;
        send!(app_sender, Action::RefreshAllViews);
        anyhow::Ok(())
    }

    fn enter_loading_state(&self, text: &str) {
        self.sync_now.set_visible(false);
        self.login_password.set_visible(false);
        self.login_browser.set_visible(false);
        self.loading_info.set_visible(true);
        self.loading_spinner.set_spinning(true);
        self.loading_label.set_text(text);
        self.loading_label
            .announce(text, gtk::AccessibleAnnouncementPriority::High);
    }

    fn set_visibilities(
        &self,
        settings_and_password: Option<(podcasts_data::sync::Settings, String)>,
    ) {
        use crate::utils::relative_time;
        if let Some((settings, password)) = settings_and_password {
            self.server.set_text(&settings.server);
            self.username.set_text(&settings.user);
            self.password.set_text(&password);
            self.active.set_active(settings.active);
            self.server_label.set_text(&settings.server);
            self.user_label.set_text(&settings.user);
            if let Some(last_sync) = settings.last_sync_local() {
                let now = chrono::Local::now();
                self.last_sync
                    .set_text(&relative_time(now.signed_duration_since(last_sync)));
                self.last_sync
                    .set_tooltip_text(Some(&format!("{}", last_sync.format("%x %X"))));
            }
            self.connection_info.set_visible(true);
            self.login_group.set_visible(false);
            self.logout.set_visible(true);
            self.sync_now.set_visible(true);
        } else {
            self.server.set_text("");
            self.username.set_text("");
            self.password.set_text("");
            self.active.set_active(false);
            self.last_sync.set_text("");
            self.server_label.set_text("");
            self.user_label.set_text("");
            self.connection_info.set_visible(false);
            self.login_group.set_visible(true);
            self.logout.set_visible(false);
            self.sync_now.set_visible(false);

            if self.method_browser.is_active() {
                self.login_browser.set_visible(false);
                self.login_browser.set_visible(true);
            } else {
                self.login_browser.set_visible(false);
                self.login_password.set_visible(true);
            }
        }
        self.loading_info.set_visible(false);
        self.loading_spinner.set_spinning(false);
    }

    async fn fetch_settings(widget_sender: Sender<WidgetAction>) {
        let result = podcasts_data::sync::Settings::fetch().await;
        send!(
            widget_sender,
            WidgetAction::GotSettings(
                result.map_err(|e| anyhow!("Failed to load settings {:#?}", e))
            )
        );
    }

    fn do_action(&self, action: WidgetAction) {
        match action {
            WidgetAction::GotSettings(result) => {
                self.set_visibilities(result.ok());
            }
            WidgetAction::LoadingMessage(message) => {
                self.enter_loading_state(&message);
            }
            WidgetAction::LogoutDone => {
                self.set_visibilities(None);
            }
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SyncPreferencesPriv {
    const NAME: &'static str = "PdSyncPreferences";
    type Type = SyncPreferences;
    type ParentType = adw::NavigationPage;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl WidgetImpl for SyncPreferencesPriv {}
impl ObjectImpl for SyncPreferencesPriv {}
impl NavigationPageImpl for SyncPreferencesPriv {}

glib::wrapper! {
    pub struct SyncPreferences(ObjectSubclass<SyncPreferencesPriv>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SyncPreferences {
    pub(crate) fn new(sender: Sender<Action>) -> Self {
        let widget: Self = glib::Object::new();
        widget.imp().init(sender);
        widget
    }
}

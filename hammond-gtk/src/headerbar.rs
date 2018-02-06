use failure::Error;
use failure::ResultExt;
use gtk;
use gtk::prelude::*;
use url::Url;

use hammond_data::Source;
use hammond_data::dbqueries;

use std::sync::Arc;
use std::sync::mpsc::Sender;

use app::Action;
use content::Content;

#[derive(Debug, Clone)]
pub struct Header {
    container: gtk::HeaderBar,
    add_toggle: gtk::MenuButton,
    switch: gtk::StackSwitcher,
    back_button: gtk::Button,
    show_title: gtk::Label,
    about_button: gtk::ModelButton,
    update_button: gtk::ModelButton,
    update_box: gtk::Box,
    update_label: gtk::Label,
    update_spinner: gtk::Spinner,
}

impl Default for Header {
    fn default() -> Header {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let header: gtk::HeaderBar = builder.get_object("headerbar").unwrap();
        let add_toggle: gtk::MenuButton = builder.get_object("add_toggle").unwrap();
        let switch: gtk::StackSwitcher = builder.get_object("switch").unwrap();
        let back_button: gtk::Button = builder.get_object("back_button").unwrap();
        let show_title: gtk::Label = builder.get_object("show_title").unwrap();
        let update_button: gtk::ModelButton = builder.get_object("update_button").unwrap();
        let update_box: gtk::Box = builder.get_object("update_notification").unwrap();
        let update_label: gtk::Label = builder.get_object("update_label").unwrap();
        let update_spinner: gtk::Spinner = builder.get_object("update_spinner").unwrap();
        let about_button: gtk::ModelButton = builder.get_object("about_button").unwrap();

        Header {
            container: header,
            add_toggle,
            switch,
            back_button,
            show_title,
            about_button,
            update_button,
            update_box,
            update_label,
            update_spinner,
        }
    }
}

// TODO: Refactor components into smaller state machines
impl Header {
    pub fn new(content: Arc<Content>, window: &gtk::Window, sender: Sender<Action>) -> Header {
        let h = Header::default();
        h.init(content, window, sender);
        h
    }

    pub fn init(&self, content: Arc<Content>, window: &gtk::Window, sender: Sender<Action>) {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let add_popover: gtk::Popover = builder.get_object("add_popover").unwrap();
        let new_url: gtk::Entry = builder.get_object("new_url").unwrap();
        let add_button: gtk::Button = builder.get_object("add_button").unwrap();
        let result_label: gtk::Label = builder.get_object("result_label").unwrap();
        self.switch.set_stack(&content.get_stack());

        new_url.connect_changed(clone!(add_button => move |url| {
            if let Err(err) = on_url_change(url, &result_label, &add_button) {
                error!("Error: {}", err);
            }
        }));

        add_button.connect_clicked(clone!(add_popover, new_url, sender => move |_| {
            if let Err(err) = on_add_bttn_clicked(&new_url, sender.clone()) {
                error!("Error: {}", err);
            }
            add_popover.hide();
        }));

        self.add_toggle.set_popover(&add_popover);

        self.update_button.connect_clicked(move |_| {
            sender.send(Action::UpdateSources(None)).unwrap();
        });

        self.about_button
            .connect_clicked(clone!(window => move |_| {
            about_dialog(&window);
        }));

        // Add the Headerbar to the window.
        window.set_titlebar(&self.container);

        let switch = &self.switch;
        let add_toggle = &self.add_toggle;
        let show_title = &self.show_title;
        self.back_button.connect_clicked(
            clone!(content, switch, add_toggle, show_title => move |back| {
            switch.show();
            add_toggle.show();
            back.hide();
            show_title.hide();
            content.get_shows().get_stack().set_visible_child_full("podcasts", gtk::StackTransitionType::SlideRight);
        }),
        );
    }

    pub fn switch_to_back(&self, title: &str) {
        self.switch.hide();
        self.add_toggle.hide();
        self.back_button.show();
        self.set_show_title(title);
        self.show_title.show();
    }

    pub fn switch_to_normal(&self) {
        self.switch.show();
        self.add_toggle.show();
        self.back_button.hide();
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

fn on_add_bttn_clicked(entry: &gtk::Entry, sender: Sender<Action>) -> Result<(), Error> {
    let url = entry.get_text().unwrap_or_default();
    let source = Source::from_url(&url).context("Failed to convert url to a Source entry.")?;
    entry.set_text("");

    sender
        .send(Action::UpdateSources(Some(source)))
        .context("App channel blew up.")?;
    Ok(())
}

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

// Totally copied it from fractal.
// https://gitlab.gnome.org/danigm/fractal/blob/503e311e22b9d7540089d735b92af8e8f93560c5/fractal-gtk/src/app.rs#L1883-1912
fn about_dialog(window: &gtk::Window) {
    // Feel free to add yourself if you contribured.
    let authors = &[
        "Jordan Petridis",
        "Julian Sparber",
        "Gabriele Musco",
        "Constantin Nickel",
    ];

    let dialog = gtk::AboutDialog::new();
    // Waiting for a logo.
    dialog.set_logo_icon_name("org.gnome.Hammond");
    dialog.set_comments("A Podcast Client for the GNOME Desktop.");
    dialog.set_copyright("© 2017, 2018 Jordan Petridis");
    dialog.set_license_type(gtk::License::Gpl30);
    dialog.set_modal(true);
    // TODO: make it show it fetches the commit hash from which it was built
    // and the version number is kept in sync automaticly
    dialog.set_version("0.3");
    dialog.set_program_name("Hammond");
    // TODO: Need a wiki page first.
    // dialog.set_website("https://wiki.gnome.org/Design/Apps/Potential/Podcasts");
    // dialog.set_website_label("Learn more about Hammond");
    dialog.set_transient_for(window);

    dialog.set_artists(&["Tobias Bernard"]);
    dialog.set_authors(authors);

    dialog.show();
}

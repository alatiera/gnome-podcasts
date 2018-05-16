use glib;
use gtk;
use gtk::prelude::*;

use failure::Error;
use failure::ResultExt;
use rayon;
use url::Url;

use hammond_data::{dbqueries, opml, Source};

use std::sync::mpsc::Sender;

use app::Action;
use stacks::Content;
use utils::{self, itunes_to_rss, refresh};

#[derive(Debug, Clone)]
// TODO: split this into smaller
pub struct Header {
    container: gtk::HeaderBar,
    add_toggle: gtk::MenuButton,
    switch: gtk::StackSwitcher,
    back: gtk::Button,
    show_title: gtk::Label,
    about: gtk::ModelButton,
    import: gtk::ModelButton,
    export: gtk::ModelButton,
    update_button: gtk::ModelButton,
    update_box: gtk::Box,
    update_label: gtk::Label,
    update_spinner: gtk::Spinner,
}

impl Default for Header {
    fn default() -> Header {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

        let header = builder.get_object("headerbar").unwrap();
        let add_toggle = builder.get_object("add_toggle").unwrap();
        let switch = builder.get_object("switch").unwrap();
        let back = builder.get_object("back").unwrap();
        let show_title = builder.get_object("show_title").unwrap();
        let import = builder.get_object("import").unwrap();
        let export = builder.get_object("export").unwrap();
        let update_button = builder.get_object("update_button").unwrap();
        let update_box = builder.get_object("update_notification").unwrap();
        let update_label = builder.get_object("update_label").unwrap();
        let update_spinner = builder.get_object("update_spinner").unwrap();
        let about = builder.get_object("about").unwrap();

        Header {
            container: header,
            add_toggle,
            switch,
            back,
            show_title,
            about,
            import,
            export,
            update_button,
            update_box,
            update_label,
            update_spinner,
        }
    }
}

// TODO: Refactor components into smaller state machines
impl Header {
    pub fn new(content: &Content, window: &gtk::Window, sender: &Sender<Action>) -> Header {
        let h = Header::default();
        h.init(content, window, &sender);
        h
    }

    pub fn init(&self, content: &Content, window: &gtk::Window, sender: &Sender<Action>) {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/headerbar.ui");

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

        self.update_button
            .connect_clicked(clone!(sender => move |_| {
                gtk::idle_add(clone!(sender => move || {
                    let s: Option<Vec<_>> = None;
                    refresh(s, sender.clone());
                    glib::Continue(false)
                }));
        }));

        self.about
            .connect_clicked(clone!(window => move |_| about_dialog(&window)));

        self.import.connect_clicked(
            clone!(window, sender => move |_| on_import_clicked(&window, &sender)),
        );

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

fn on_import_clicked(window: &gtk::Window, sender: &Sender<Action>) {
    use glib::translate::ToGlib;
    use gtk::{FileChooserAction, FileChooserDialog, FileFilter, ResponseType};

    // let dialog = FileChooserDialog::new(title, Some(&window), FileChooserAction::Open);
    // TODO: It might be better to use a FileChooserNative widget.
    // Create the FileChooser Dialog
    let dialog = FileChooserDialog::with_buttons(
        Some("Select the file from which to you want to Import Shows."),
        Some(window),
        FileChooserAction::Open,
        &[
            ("_Cancel", ResponseType::Cancel),
            ("_Open", ResponseType::Accept),
        ],
    );

    // Do not show hidden(.thing) files
    dialog.set_show_hidden(false);

    // Set a filter to show only xml files
    let filter = FileFilter::new();
    FileFilterExt::set_name(&filter, Some("OPML file"));
    filter.add_mime_type("application/xml");
    filter.add_mime_type("text/xml");
    dialog.add_filter(&filter);

    dialog.connect_response(clone!(sender => move |dialog, resp| {
        debug!("Dialong Response {}", resp);
        if resp == ResponseType::Accept.to_glib() {
            // TODO: Show an in-app notifictaion if the file can not be accessed
            if let Some(filename) = dialog.get_filename() {
                debug!("File selected: {:?}", filename);

                rayon::spawn(clone!(sender => move || {
                    // Parse the file and import the feeds
                    if let Ok(sources) = opml::import_from_file(filename) {
                        // Refresh the succesfully parsed feeds to index them
                        utils::refresh(Some(sources), sender)
                    } else {
                        let text = String::from("Failed to parse the Imported file");
                        sender.send(Action::ErrorNotification(text))
                            .map_err(|err| error!("Action Sender: {}", err))
                            .ok();
                    }
                }))
            } else {
                let text = String::from("Selected File could not be accessed.");
                sender.send(Action::ErrorNotification(text))
                    .map_err(|err| error!("Action Sender: {}", err))
                    .ok();
            }
        }

        dialog.destroy();
    }));

    dialog.run();
}

// Totally copied it from fractal.
// https://gitlab.gnome.org/danigm/fractal/blob/503e311e22b9d7540089d735b92af8e8f93560c5/fractal-gtk/src/app.rs#L1883-1912
fn about_dialog(window: &gtk::Window) {
    // Feel free to add yourself if you contribured.
    let authors = &[
        "Constantin Nickel",
        "Gabriele Musco",
        "James Wykeham-Martin",
        "Jordan Petridis",
        "Julian Sparber",
        "Rowan Lewis",
    ];

    let dialog = gtk::AboutDialog::new();
    // Waiting for a logo.
    // dialog.set_logo_icon_name("org.gnome.Hammond");
    dialog.set_logo_icon_name("multimedia-player");
    dialog.set_comments("Podcast Client for the GNOME Desktop.");
    dialog.set_copyright("© 2017, 2018 Jordan Petridis");
    dialog.set_license_type(gtk::License::Gpl30);
    dialog.set_modal(true);
    // TODO: make it show it fetches the commit hash from which it was built
    // and the version number is kept in sync automaticly
    dialog.set_version("0.3.2");
    dialog.set_program_name("Hammond");
    // TODO: Need a wiki page first.
    // dialog.set_website("https://wiki.gnome.org/Design/Apps/Potential/Podcasts");
    // dialog.set_website_label("Learn more about Hammond");
    dialog.set_transient_for(window);

    dialog.set_artists(&["Tobias Bernard"]);
    dialog.set_authors(authors);

    dialog.show();
}

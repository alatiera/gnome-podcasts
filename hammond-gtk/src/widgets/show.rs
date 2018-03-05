use dissolve;
use failure::Error;
use gtk;
use gtk::prelude::*;
use open;

use hammond_data::Podcast;
use hammond_data::dbqueries;
use hammond_data::utils::{delete_show, replace_extra_spaces};

use app::Action;
use utils::get_pixbuf_from_path;
use widgets::episode::episodes_listbox;

use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

#[derive(Debug, Clone)]
pub struct ShowWidget {
    pub container: gtk::Box,
    scrolled_window: gtk::ScrolledWindow,
    cover: gtk::Image,
    description: gtk::Label,
    link: gtk::Button,
    settings: gtk::MenuButton,
    unsub: gtk::Button,
    episodes: gtk::Frame,
}

impl Default for ShowWidget {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/show_widget.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let scrolled_window: gtk::ScrolledWindow = builder.get_object("scrolled_window").unwrap();
        let episodes: gtk::Frame = builder.get_object("episodes").unwrap();

        let cover: gtk::Image = builder.get_object("cover").unwrap();
        let description: gtk::Label = builder.get_object("description").unwrap();
        let unsub: gtk::Button = builder.get_object("unsub_button").unwrap();
        let link: gtk::Button = builder.get_object("link_button").unwrap();
        let settings: gtk::MenuButton = builder.get_object("settings_button").unwrap();

        ShowWidget {
            container,
            scrolled_window,
            cover,
            description,
            unsub,
            link,
            settings,
            episodes,
        }
    }
}

impl ShowWidget {
    pub fn new(pd: Arc<Podcast>, sender: Sender<Action>) -> ShowWidget {
        let pdw = ShowWidget::default();
        pdw.init(pd, sender);
        pdw
    }

    pub fn init(&self, pd: Arc<Podcast>, sender: Sender<Action>) {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/show_widget.ui");

        // Hacky workaround so the pd.id() can be retrieved from the `ShowStack`.
        WidgetExt::set_name(&self.container, &pd.id().to_string());

        self.unsub
            .connect_clicked(clone!(pd, sender => move |bttn| {
                if let Err(err) = on_unsub_button_clicked(pd.clone(), bttn, sender.clone()) {
                    error!("Error: {}", err);
                }
        }));

        self.setup_listbox(&pd, sender.clone());
        self.set_description(pd.description());

        if let Err(err) = self.set_cover(pd.clone()) {
            error!("Failed to set a cover: {}", err)
        }

        let link = pd.link().to_owned();
        self.link.set_tooltip_text(Some(link.as_str()));
        self.link.connect_clicked(move |_| {
            info!("Opening link: {}", &link);
            if let Err(err) = open::that(&link) {
                error!("Failed to open link: {}", &link);
                error!("Error: {}", err);
            }
        });

        let show_menu: gtk::Popover = builder.get_object("show_menu").unwrap();
        let mark_all: gtk::ModelButton = builder.get_object("mark_all_watched").unwrap();

        mark_all.connect_clicked(clone!(pd, sender => move |_| {
            if let Err(err) = on_played_button_clicked(&pd, sender.clone()) {
                error!("Failed to mark all episodes as watched: {}", err);
            }
        }));
        self.settings.set_popover(&show_menu);
    }

    /// Populate the listbox with the shows episodes.
    fn setup_listbox(&self, pd: &Podcast, sender: Sender<Action>) {
        let listbox = episodes_listbox(pd, sender.clone());
        listbox.ok().map(|l| self.episodes.add(&l));
    }

    /// Set the show cover.
    fn set_cover(&self, pd: Arc<Podcast>) -> Result<(), Error> {
        let image = get_pixbuf_from_path(&pd.into(), 128)?;
        self.cover.set_from_pixbuf(&image);
        Ok(())
    }

    /// Set the descripton text.
    fn set_description(&self, text: &str) {
        // TODO: Temporary solution until we render html urls/bold/italic probably with
        // markup.
        let desc = dissolve::strip_html_tags(text).join(" ");
        self.description.set_text(&replace_extra_spaces(&desc));
    }

    /// Set scrolled window vertical adjustment.
    pub fn set_vadjustment(&self, vadjustment: &gtk::Adjustment) {
        self.scrolled_window.set_vadjustment(vadjustment)
    }
}

fn on_unsub_button_clicked(
    pd: Arc<Podcast>,
    unsub_button: &gtk::Button,
    sender: Sender<Action>,
) -> Result<(), Error> {
    // hack to get away without properly checking for none.
    // if pressed twice would panic.
    unsub_button.hide();
    // Spawn a thread so it won't block the ui.
    thread::spawn(move || {
        if let Err(err) = delete_show(&pd) {
            error!("Something went wrong trying to remove {}", pd.title());
            error!("Error: {}", err);
        }
    });

    sender.send(Action::HeaderBarNormal)?;
    sender.send(Action::ShowShowsAnimated)?;
    // Queue a refresh after the switch to avoid blocking the db.
    sender.send(Action::RefreshShowsView)?;
    sender.send(Action::RefreshEpisodesView)?;

    Ok(())
}

fn on_played_button_clicked(pd: &Podcast, sender: Sender<Action>) -> Result<(), Error> {
    dbqueries::update_none_to_played_now(pd)?;
    sender.send(Action::RefreshWidget)?;
    Ok(())
}

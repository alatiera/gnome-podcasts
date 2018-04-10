use failure::Error;
// use glib;
use gtk;
use gtk::prelude::*;
use html2pango::markup_from_raw;
use open;

use hammond_data::Podcast;
use hammond_data::dbqueries;

use app::Action;
use utils::set_image_from_path;
use widgets::episode::episodes_listbox;

use std::sync::Arc;
use std::sync::mpsc::Sender;

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

        self.setup_listbox(pd.clone(), sender.clone());
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

        let episodes = self.episodes.clone();
        mark_all.connect_clicked(clone!(pd, sender => move |_| {
            on_played_button_clicked(
                pd.clone(),
                &episodes,
                sender.clone()
            )
        }));
        self.settings.set_popover(&show_menu);
    }

    /// Populate the listbox with the shows episodes.
    fn setup_listbox(&self, pd: Arc<Podcast>, sender: Sender<Action>) {
        let listbox = episodes_listbox(pd, sender.clone());
        listbox.ok().map(|l| self.episodes.add(&l));
    }

    /// Set the show cover.
    fn set_cover(&self, pd: Arc<Podcast>) -> Result<(), Error> {
        set_image_from_path(&self.cover, Arc::new(pd.into()), 128)
    }

    /// Set the descripton text.
    fn set_description(&self, text: &str) {
        self.description.set_markup(&markup_from_raw(text));
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
    sender.send(Action::RemoveShow(pd))?;

    sender.send(Action::HeaderBarNormal)?;
    sender.send(Action::ShowShowsAnimated)?;
    // Queue a refresh after the switch to avoid blocking the db.
    sender.send(Action::RefreshShowsView)?;
    sender.send(Action::RefreshEpisodesView)?;

    Ok(())
}

fn on_played_button_clicked(pd: Arc<Podcast>, episodes: &gtk::Frame, sender: Sender<Action>) {
    if dim_titles(episodes).is_none() {
        error!("Something went horribly wrong when dimming the titles.");
        warn!("RUN WHILE YOU STILL CAN!");
    }

    sender.send(Action::MarkAllPlayerNotification(pd)).unwrap();
}

pub fn mark_all_watched(pd: &Podcast, sender: Sender<Action>) -> Result<(), Error> {
    dbqueries::update_none_to_played_now(pd)?;
    sender.send(Action::RefreshWidgetIfVis)?;
    sender.send(Action::RefreshEpisodesView)?;
    Ok(())
}

// Ideally if we had a custom widget this would have been as simple as:
// `for row in listbox { ep = row.get_episode(); ep.dim_title(); }`
// But now I can't think of a better way to do it than hardcoding the title
// position relative to the EpisodeWidget container gtk::Box.
fn dim_titles(episodes: &gtk::Frame) -> Option<()> {
    let listbox = episodes
        .get_children()
        .remove(0)
        .downcast::<gtk::ListBox>()
        .ok()?;
    let children = listbox.get_children();

    for row in children {
        let row = row.downcast::<gtk::ListBoxRow>().ok()?;
        let container = row.get_children().remove(0).downcast::<gtk::Box>().ok()?;
        let foo = container
            .get_children()
            .remove(0)
            .downcast::<gtk::Box>()
            .ok()?;
        let bar = foo.get_children().remove(0).downcast::<gtk::Box>().ok()?;
        let baz = bar.get_children().remove(0).downcast::<gtk::Box>().ok()?;
        let title = baz.get_children().remove(0).downcast::<gtk::Label>().ok()?;

        title.get_style_context().map(|c| c.add_class("dim-label"));
    }
    Some(())
}

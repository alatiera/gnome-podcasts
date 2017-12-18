use gtk;
use gtk::prelude::*;

use widgets::episode::EpisodeWidget;

#[derive(Debug, Clone)]
pub struct EpisodesView {
    pub container: gtk::Box,
    frame_parent: gtk::Box,
}

impl Default for EpisodesView {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/episodes_view.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let frame_parent: gtk::Box = builder.get_object("frame_parent").unwrap();

        EpisodesView {
            container,
            frame_parent,
        }
    }
}

#[derive(Debug, Clone)]
struct EpisodesViewWidget {
    container: gtk::Box,
    image: gtk::Image,
    episode: gtk::Box,
}

impl Default for EpisodesViewWidget {
    fn default() -> Self {
        let builder =
            gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/episodes_view_widget.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let image: gtk::Image = builder.get_object("cover").unwrap();
        let ep = EpisodeWidget::default();
        container.add(&ep.container);

        EpisodesViewWidget {
            container,
            image,
            episode: ep.container,
        }
    }
}

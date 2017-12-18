use gtk;
use gtk::prelude::*;

use hammond_data::dbqueries;
use hammond_data::EpisodeWidgetQuery;

use widgets::episode::EpisodeWidget;

use std::rc::Rc;

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

impl EpisodesView {
    pub fn new() -> Rc<EpisodesView> {
        let view = EpisodesView::default();

        let episodes = dbqueries::get_episodeswidgets_with_limit(100).unwrap();
        let frame = gtk::Frame::new("Recent Episodes");
        let list = gtk::ListBox::new();

        view.frame_parent.add(&frame);
        frame.add(&list);

        list.set_vexpand(false);
        list.set_hexpand(false);
        list.set_visible(true);
        list.set_selection_mode(gtk::SelectionMode::None);

        episodes.into_iter().for_each(|mut ep| {
            let viewep = EpisodesViewWidget::new(&mut ep);
            list.add(&viewep.container);

            let sep = gtk::Separator::new(gtk::Orientation::Vertical);
            sep.set_sensitive(false);
            sep.set_can_focus(false);

            list.add(&sep);
            sep.show()
        });

        Rc::new(view)
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
        container.pack_start(&ep.container, true, true, 5);

        EpisodesViewWidget {
            container,
            image,
            episode: ep.container,
        }
    }
}

impl EpisodesViewWidget {
    fn new(episode: &mut EpisodeWidgetQuery) -> EpisodesViewWidget {
        let builder =
            gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/episodes_view_widget.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let image: gtk::Image = builder.get_object("cover").unwrap();
        let ep = EpisodeWidget::new(episode);
        container.pack_start(&ep.container, true, true, 5);

        EpisodesViewWidget {
            container,
            image,
            episode: ep.container,
        }
    }
}

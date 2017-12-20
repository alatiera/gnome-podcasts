use gtk;
use gtk::prelude::*;
use chrono::prelude::*;

use hammond_data::dbqueries;
use hammond_data::EpisodeWidgetQuery;

use widgets::episode::EpisodeWidget;
use utils::get_pixbuf_from_path_64;

use std::rc::Rc;

#[derive(Debug, Clone)]
enum ListSplit {
    Today,
    Yday,
    Week,
    Month,
}

#[derive(Debug, Clone)]
pub struct EpisodesView {
    pub container: gtk::Box,
    frame_parent: gtk::Box,
    today_box: gtk::ListBox,
    yday_box: gtk::ListBox,
    week_box: gtk::ListBox,
    month_box: gtk::ListBox,
    today_label: gtk::Label,
    yday_label: gtk::Label,
    week_label: gtk::Label,
    month_label: gtk::Label,
}

impl Default for EpisodesView {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/episodes_view.ui");
        let container: gtk::Box = builder.get_object("container").unwrap();
        let frame_parent: gtk::Box = builder.get_object("frame_parent").unwrap();
        let today_box: gtk::ListBox = builder.get_object("today_box").unwrap();
        let yday_box: gtk::ListBox = builder.get_object("yday_box").unwrap();
        let week_box: gtk::ListBox = builder.get_object("week_box").unwrap();
        let month_box: gtk::ListBox = builder.get_object("month_box").unwrap();
        let today_label: gtk::Label = builder.get_object("today_label").unwrap();
        let yday_label: gtk::Label = builder.get_object("yday_label").unwrap();
        let week_label: gtk::Label = builder.get_object("week_label").unwrap();
        let month_label: gtk::Label = builder.get_object("month_label").unwrap();

        EpisodesView {
            container,
            frame_parent,
            today_box,
            yday_box,
            week_box,
            month_box,
            today_label,
            yday_label,
            week_label,
            month_label,
        }
    }
}

impl EpisodesView {
    pub fn new() -> Rc<EpisodesView> {
        let view = EpisodesView::default();
        let episodes = dbqueries::get_episodes_widgets_with_limit(100).unwrap();
        let now_utc = Utc::now().timestamp() as i32;

        episodes.into_iter().for_each(|mut ep| {
            let viewep = EpisodesViewWidget::new(&mut ep);
            let sep = gtk::Separator::new(gtk::Orientation::Vertical);
            sep.set_sensitive(false);
            sep.set_can_focus(false);

            let t = split(now_utc, ep.epoch());
            match t {
                ListSplit::Today => {
                    view.today_box.add(&viewep.container);
                    view.today_box.add(&sep)
                }
                ListSplit::Yday => {
                    view.yday_box.add(&viewep.container);
                    view.yday_box.add(&sep)
                }
                ListSplit::Week => {
                    view.week_box.add(&viewep.container);
                    view.week_box.add(&sep)
                }
                _ => {
                    view.month_box.add(&viewep.container);
                    view.month_box.add(&sep)
                }
            }

            sep.show()
        });

        view.container.show_all();
        Rc::new(view)
    }
}

fn split(now_utc: i32, epoch: i32) -> ListSplit {
    let t = now_utc - epoch;

    if t < 86_400 {
        ListSplit::Today
    } else if t < 172_800 {
        ListSplit::Yday
    } else if t < 604_800 {
        ListSplit::Week
    } else {
        ListSplit::Month
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

        // FIXME:
        if let Ok(pd) = dbqueries::get_podcast_from_id(episode.podcast_id()) {
            let img = get_pixbuf_from_path_64(&pd);
            if let Some(i) = img {
                image.set_from_pixbuf(&i);
            }
        }

        let ep = EpisodeWidget::new(episode);
        container.pack_start(&ep.container, true, true, 5);

        EpisodesViewWidget {
            container,
            image,
            episode: ep.container,
        }
    }
}

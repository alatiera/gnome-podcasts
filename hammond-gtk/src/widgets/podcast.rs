use gtk::prelude::*;
use gtk;
use diesel::Identifiable;

use std::fs;

use hammond_data::dbqueries;
use hammond_data::Podcast;
use hammond_downloader::downloader;

use widgets::episode::episodes_listbox;
use utils::get_pixbuf_from_path;
use content::ShowStack;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct PodcastWidget {
    pub container: gtk::Box,
    cover: gtk::Image,
    title: gtk::Label,
    description: gtk::TextView,
    view: gtk::Viewport,
    unsub: gtk::Button,
    played: gtk::Button,
}

impl PodcastWidget {
    pub fn new() -> PodcastWidget {
        // Adapted from gnome-music AlbumWidget
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/podcast_widget.ui");
        let container: gtk::Box = builder.get_object("podcast_widget").unwrap();

        let cover: gtk::Image = builder.get_object("cover").unwrap();
        let title: gtk::Label = builder.get_object("title_label").unwrap();
        let description: gtk::TextView = builder.get_object("desc_text_view").unwrap();
        let view: gtk::Viewport = builder.get_object("view").unwrap();
        let unsub: gtk::Button = builder.get_object("unsub_button").unwrap();
        let played: gtk::Button = builder.get_object("mark_all_played_button").unwrap();

        PodcastWidget {
            container,
            cover,
            title,
            description,
            view,
            unsub,
            played,
        }
    }

    pub fn new_initialized(shows: Rc<ShowStack>, pd: &Podcast) -> PodcastWidget {
        let pdw = PodcastWidget::new();
        pdw.init(shows, pd);
        pdw
    }

    pub fn init(&self, shows: Rc<ShowStack>, pd: &Podcast) {
        WidgetExt::set_name(&self.container, &pd.id().to_string());

        // TODO: should spawn a thread to avoid locking the UI probably.
        self.unsub.connect_clicked(clone!(shows, pd => move |bttn| {
            on_unsub_button_clicked(shows.clone(), &pd, bttn);
        }));

        self.title.set_text(pd.title());
        let listbox = episodes_listbox(pd);
        if let Ok(l) = listbox {
            self.view.add(&l);
        }

        {
            let buff = self.description.get_buffer().unwrap();
            buff.set_text(pd.description());
        }

        let img = get_pixbuf_from_path(pd);
        if let Some(i) = img {
            self.cover.set_from_pixbuf(&i);
        }

        self.played.connect_clicked(clone!(shows, pd => move |_| {
            on_played_button_clicked(shows.clone(), &pd);
        }));

        self.show_played_button(pd);
    }

    fn show_played_button(&self, pd: &Podcast) {
        let new_episodes = dbqueries::get_pd_unplayed_episodes(pd);

        if let Ok(n) = new_episodes {
            if !n.is_empty() {
                self.played.show()
            }
        }
    }
}

fn on_unsub_button_clicked(shows: Rc<ShowStack>, pd: &Podcast, unsub_button: &gtk::Button) {
    let res = dbqueries::remove_feed(pd);
    if res.is_ok() {
        info!("{} was removed succesfully.", pd.title());
        // hack to get away without properly checking for none.
        // if pressed twice would panic.
        unsub_button.hide();

        let dl_fold = downloader::get_download_folder(pd.title());
        if let Ok(fold) = dl_fold {
            let res3 = fs::remove_dir_all(&fold);
            if res3.is_ok() {
                info!("All the content at, {} was removed succesfully", &fold);
            }
        };
    }
    shows.switch_podcasts_animated();
    shows.update_podcasts();
}

fn on_played_button_clicked(shows: Rc<ShowStack>, pd: &Podcast) {
    let _ = dbqueries::update_none_to_played_now(pd);

    shows.update_widget();
}

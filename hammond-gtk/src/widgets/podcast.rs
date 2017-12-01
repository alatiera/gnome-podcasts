use gtk::prelude::*;
use gtk;

use std::fs;

use hammond_data::dbqueries;
use hammond_data::Podcast;
use hammond_downloader::downloader;

use widgets::episode::episodes_listbox;
use views::podcasts::update_podcasts_view;
use utils::get_pixbuf_from_path;

#[derive(Debug)]
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

    pub fn init(&self, stack: &gtk::Stack, pd: &Podcast) {
        // TODO: should spawn a thread to avoid locking the UI probably.
        self.unsub.connect_clicked(clone!(stack, pd => move |bttn| {
            on_unsub_button_clicked(&stack, &pd, bttn);
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

        self.played.connect_clicked(clone!(stack, pd => move |_| {
            on_played_button_clicked(&stack, &pd);
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

// Note: Stack manipulation
fn on_unsub_button_clicked(stack: &gtk::Stack, pd: &Podcast, unsub_button: &gtk::Button) {
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
    stack.set_visible_child_name("podcasts");
    update_podcasts_view(stack);
}

fn on_played_button_clicked(stack: &gtk::Stack, pd: &Podcast) {
    let _ = dbqueries::update_none_to_played_now(pd);

    update_podcast_widget(stack, pd);
}

// Note: Stack manipulation
pub fn update_podcast_widget(stack: &gtk::Stack, pd: &Podcast) {
    let old = stack.get_child_by_name("widget").unwrap();
    let pdw = PodcastWidget::new();
    pdw.init(stack, pd);
    let vis = stack.get_visible_child_name().unwrap();

    stack.remove(&old);
    stack.add_named(&pdw.container, "widget");
    stack.set_visible_child_name(&vis);
    old.destroy();
}

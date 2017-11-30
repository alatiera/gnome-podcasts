use gtk::prelude::*;
use gtk;
use gdk_pixbuf::Pixbuf;

use std::fs;

use hammond_data::dbqueries;
use hammond_data::Podcast;
use hammond_downloader::downloader;

use widgets::episode::episodes_listbox;
use views::podcasts::update_podcasts_view;

#[derive(Debug)]
struct PodcastWidget {
    container: gtk::Box,
    cover: gtk::Image,
    title: gtk::Label,
    description: gtk::TextView,
    view: gtk::Viewport,
    unsub: gtk::Button,
    played: gtk::Button,
}

impl PodcastWidget {
    fn new() -> PodcastWidget {
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

        show_played_button(pd, &self.played);
    }
}

pub fn podcast_widget(stack: &gtk::Stack, pd: &Podcast) -> gtk::Box {
    // Adapted from gnome-music AlbumWidget
    let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/podcast_widget.ui");
    let pd_widget: gtk::Box = builder.get_object("podcast_widget").unwrap();

    let cover: gtk::Image = builder.get_object("cover").unwrap();
    let title_label: gtk::Label = builder.get_object("title_label").unwrap();
    let desc_text_view: gtk::TextView = builder.get_object("desc_text_view").unwrap();
    let view: gtk::Viewport = builder.get_object("view").unwrap();
    let unsub_button: gtk::Button = builder.get_object("unsub_button").unwrap();
    let played_button: gtk::Button = builder.get_object("mark_all_played_button").unwrap();

    // TODO: should spawn a thread to avoid locking the UI probably.
    unsub_button.connect_clicked(clone!(stack, pd => move |bttn| {
        on_unsub_button_clicked(&stack, &pd, bttn);
    }));

    title_label.set_text(pd.title());
    let listbox = episodes_listbox(pd);
    if let Ok(l) = listbox {
        view.add(&l);
    }

    {
        let buff = desc_text_view.get_buffer().unwrap();
        buff.set_text(pd.description());
    }

    let img = get_pixbuf_from_path(pd);
    if let Some(i) = img {
        cover.set_from_pixbuf(&i);
    }

    played_button.connect_clicked(clone!(stack, pd => move |_| {
        on_played_button_clicked(&stack, &pd);
    }));

    show_played_button(pd, &played_button);

    pd_widget
}

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
    stack.set_visible_child_name("fb_parent");
    update_podcasts_view(stack);
}

fn on_played_button_clicked(stack: &gtk::Stack, pd: &Podcast) {
    let _ = dbqueries::update_none_to_played_now(pd);

    update_podcast_widget(stack, pd);
}

fn show_played_button(pd: &Podcast, played_button: &gtk::Button) {
    let new_episodes = dbqueries::get_pd_unplayed_episodes(pd);

    if let Ok(n) = new_episodes {
        if !n.is_empty() {
            played_button.show()
        }
    }
}

pub fn get_pixbuf_from_path(pd: &Podcast) -> Option<Pixbuf> {
    let img_path = downloader::cache_image(pd);
    if let Some(i) = img_path {
        Pixbuf::new_from_file_at_scale(&i, 256, 256, true).ok()
    } else {
        None
    }
}

pub fn setup_podcast_widget(stack: &gtk::Stack) {
    let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/podcast_widget.ui");
    let pd_widget: gtk::Box = builder.get_object("podcast_widget").unwrap();

    stack.add_named(&pd_widget, "pdw");
}

pub fn update_podcast_widget(stack: &gtk::Stack, pd: &Podcast) {
    let old = stack.get_child_by_name("pdw").unwrap();
    let pdw = podcast_widget(stack, pd);
    let vis = stack.get_visible_child_name().unwrap();

    stack.remove(&old);
    stack.add_named(&pdw, "pdw");
    stack.set_visible_child_name(&vis);
    old.destroy();
}

#[cfg(test)]
mod tests {
    use hammond_data::Source;
    use hammond_data::feed::index;
    use diesel::Identifiable;
    use super::*;

    #[test]
    fn test_get_pixbuf_from_path() {
        let url = "http://www.newrustacean.com/feed.xml";

        // Create and index a source
        let source = Source::from_url(url).unwrap();
        // Copy it's id
        let sid = source.id().clone();

        // Convert Source it into a Feed and index it
        let feed = source.into_feed().unwrap();
        index(vec![feed]);

        // Get the Podcast
        let pd = dbqueries::get_podcast_from_source_id(sid).unwrap();
        let pxbuf = get_pixbuf_from_path(&pd);
        assert!(pxbuf.is_some());
    }
}

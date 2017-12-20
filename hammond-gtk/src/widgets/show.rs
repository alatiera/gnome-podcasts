use gtk::prelude::*;
use gtk;
use diesel::Identifiable;
use open;
use dissolve;

use hammond_data::dbqueries;
use hammond_data::Podcast;
use hammond_data::utils::replace_extra_spaces;
use hammond_downloader::downloader;

use widgets::episode::episodes_listbox;
use utils::get_pixbuf_from_path;
use content::ShowStack;
use headerbar::Header;

use std::rc::Rc;
use std::fs;

#[derive(Debug, Clone)]
pub struct ShowWidget {
    pub container: gtk::Box,
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
        let episodes: gtk::Frame = builder.get_object("episodes").unwrap();

        let cover: gtk::Image = builder.get_object("cover").unwrap();
        let description: gtk::Label = builder.get_object("description").unwrap();
        let unsub: gtk::Button = builder.get_object("unsub_button").unwrap();
        let link: gtk::Button = builder.get_object("link_button").unwrap();
        let settings: gtk::MenuButton = builder.get_object("settings_button").unwrap();

        ShowWidget {
            container,
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
    pub fn new(shows: Rc<ShowStack>, header: Rc<Header>, pd: &Podcast) -> ShowWidget {
        let pdw = ShowWidget::default();
        pdw.init(shows, header, pd);
        pdw
    }

    pub fn init(&self, shows: Rc<ShowStack>, header: Rc<Header>, pd: &Podcast) {
        WidgetExt::set_name(&self.container, &pd.id().to_string());

        // TODO: should spawn a thread to avoid locking the UI probably.
        self.unsub.connect_clicked(clone!(shows, pd => move |bttn| {
            on_unsub_button_clicked(shows.clone(), &pd, bttn);
            header.switch_to_normal();
        }));

        let listbox = episodes_listbox(pd);
        if let Ok(l) = listbox {
            self.episodes.add(&l);
        }

        // TODO: Temporary solution until we render html urls/bold/italic probably with markup.
        let desc = dissolve::strip_html_tags(pd.description()).join(" ");
        self.description.set_text(&replace_extra_spaces(&desc));

        let img = get_pixbuf_from_path(&pd.clone().into(), 128);
        if let Some(i) = img {
            self.cover.set_from_pixbuf(&i);
        }

        let link = pd.link().to_owned();
        self.link.connect_clicked(move |_| {
            info!("Opening link: {}", &link);
            let _ = open::that(&link);
        });

        // self.played.connect_clicked(clone!(shows, pd => move |_| {
        //     on_played_button_clicked(shows.clone(), &pd);
        // }));
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

#[allow(dead_code)]
fn on_played_button_clicked(shows: Rc<ShowStack>, pd: &Podcast) {
    let _ = dbqueries::update_none_to_played_now(pd);

    shows.update_widget();
}

use gtk;
use gtk::prelude::*;
use diesel::associations::Identifiable;

use hammond_data::dbqueries;
use hammond_data::Podcast;

use utils::get_pixbuf_from_path;
use content::ShowStack;
use headerbar::Header;

use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ShowsPopulated {
    pub container: gtk::Box,
    flowbox: gtk::FlowBox,
    viewport: gtk::Viewport,
}

impl Default for ShowsPopulated {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/shows_view.ui");
        let container: gtk::Box = builder.get_object("fb_parent").unwrap();
        let flowbox: gtk::FlowBox = builder.get_object("flowbox").unwrap();
        let viewport: gtk::Viewport = builder.get_object("viewport").unwrap();

        ShowsPopulated {
            container,
            flowbox,
            viewport,
        }
    }
}

impl ShowsPopulated {
    pub fn new(show: Rc<ShowStack>, header: Rc<Header>) -> ShowsPopulated {
        let pop = ShowsPopulated::default();
        pop.init(show, header);
        pop
    }

    pub fn init(&self, show: Rc<ShowStack>, header: Rc<Header>) {
        use gtk::WidgetExt;

        // TODO: handle unwraps.
        // TODO: implement back button.
        self.flowbox
            .connect_child_activated(clone!(show => move |_, child| {
            // This is such an ugly hack...
            let id = WidgetExt::get_name(child).unwrap().parse::<i32>().unwrap();
            let pd = dbqueries::get_podcast_from_id(id).unwrap();

            show.replace_widget(&pd);
            header.switch_to_back(pd.title());
            show.switch_widget_animated();
        }));
        // Populate the flowbox with the Podcasts.
        self.populate_flowbox();
    }

    fn populate_flowbox(&self) {
        let podcasts = dbqueries::get_podcasts();

        if let Ok(pds) = podcasts {
            pds.iter().for_each(|parent| {
                let flowbox_child = ShowsChild::new(parent);
                self.flowbox.add(&flowbox_child.child);
            });
            self.flowbox.show_all();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.flowbox.get_children().is_empty()
    }
}

#[derive(Debug)]
struct ShowsChild {
    container: gtk::Box,
    cover: gtk::Image,
    child: gtk::FlowBoxChild,
}

impl Default for ShowsChild {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/shows_child.ui");

        let container: gtk::Box = builder.get_object("fb_child").unwrap();
        let cover: gtk::Image = builder.get_object("pd_cover").unwrap();

        let child = gtk::FlowBoxChild::new();
        child.add(&container);

        ShowsChild {
            container,
            cover,
            child,
        }
    }
}

impl ShowsChild {
    pub fn new(pd: &Podcast) -> ShowsChild {
        let child = ShowsChild::default();
        child.init(pd);
        child
    }

    fn init(&self, pd: &Podcast) {
        self.container.set_tooltip_text(pd.title());

        let cover = get_pixbuf_from_path(pd);
        if let Some(img) = cover {
            self.cover.set_from_pixbuf(&img);
        };

        WidgetExt::set_name(&self.child, &pd.id().to_string());
    }
}

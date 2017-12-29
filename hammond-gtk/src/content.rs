use gtk;
use gtk::prelude::*;

use hammond_data::Podcast;
use hammond_data::dbqueries;

use views::shows::ShowsPopulated;
use views::empty::EmptyView;
use views::episodes::EpisodesView;

use widgets::show::ShowWidget;
use headerbar::Header;

use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Content {
    stack: gtk::Stack,
    shows: Rc<ShowStack>,
    episodes: Rc<EpisodeStack>,
}

impl Content {
    pub fn new(header: Rc<Header>) -> Rc<Content> {
        let stack = gtk::Stack::new();
        let episodes = EpisodeStack::new();
        let shows = ShowStack::new(header, episodes.clone());

        stack.add_titled(&episodes.stack, "episodes", "Episodes");
        stack.add_titled(&shows.stack, "shows", "Shows");

        Rc::new(Content {
            stack,
            shows,
            episodes,
        })
    }

    pub fn update(&self) {
        self.update_shows_view();
        self.update_episode_view();
    }

    pub fn update_episode_view(&self) {
        self.episodes.update();
    }

    pub fn update_shows_view(&self) {
        self.shows.update();
    }

    pub fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub fn get_shows(&self) -> Rc<ShowStack> {
        self.shows.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ShowStack {
    stack: gtk::Stack,
    header: Rc<Header>,
    epstack: Rc<EpisodeStack>,
}

impl ShowStack {
    fn new(header: Rc<Header>, epstack: Rc<EpisodeStack>) -> Rc<ShowStack> {
        let stack = gtk::Stack::new();

        let show = Rc::new(ShowStack {
            stack,
            header: header.clone(),
            epstack,
        });

        let pop = ShowsPopulated::new(show.clone(), header);
        let widget = ShowWidget::default();
        let empty = EmptyView::new();

        show.stack.add_named(&pop.container, "podcasts");
        show.stack.add_named(&widget.container, "widget");
        show.stack.add_named(&empty.container, "empty");

        if pop.is_empty() {
            show.stack.set_visible_child_name("empty")
        } else {
            show.stack.set_visible_child_name("podcasts")
        }

        show
    }

    // fn is_empty(&self) -> bool {
    //     self.podcasts.is_empty()
    // }

    pub fn update(&self) {
        self.update_podcasts();
        self.update_widget();
    }

    pub fn update_podcasts(&self) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("podcasts").unwrap();

        let pop = ShowsPopulated::default();
        pop.init(Rc::new(self.clone()), self.header.clone());

        self.stack.remove(&old);
        self.stack.add_named(&pop.container, "podcasts");

        if pop.is_empty() {
            self.stack.set_visible_child_name("empty");
        } else if vis != "empty" {
            self.stack.set_visible_child_name(&vis);
        } else {
            self.stack.set_visible_child_name("podcasts");
        }

        old.destroy();
    }

    pub fn replace_widget(&self, pd: &Podcast) {
        let old = self.stack.get_child_by_name("widget").unwrap();
        let new = ShowWidget::new(
            Rc::new(self.clone()),
            self.epstack.clone(),
            self.header.clone(),
            pd,
        );

        self.stack.remove(&old);
        self.stack.add_named(&new.container, "widget");
    }

    pub fn update_widget(&self) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("widget").unwrap();

        let id = WidgetExt::get_name(&old).unwrap();
        if id == "GtkBox" {
            return;
        }

        let pd = dbqueries::get_podcast_from_id(id.parse::<i32>().unwrap());
        if let Ok(pd) = pd {
            self.replace_widget(&pd);
            self.stack.set_visible_child_name(&vis);
            old.destroy();
        }
    }

    pub fn switch_podcasts_animated(&self) {
        self.stack
            .set_visible_child_full("podcasts", gtk::StackTransitionType::SlideRight);
    }

    pub fn switch_widget_animated(&self) {
        self.stack
            .set_visible_child_full("widget", gtk::StackTransitionType::SlideLeft)
    }

    pub fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }
}

#[derive(Debug, Clone)]
pub struct EpisodeStack {
    // populated: RecentEpisodes,
    // empty: EmptyView,
    stack: gtk::Stack,
}

impl EpisodeStack {
    fn new() -> Rc<EpisodeStack> {
        let episodes = EpisodesView::new();
        let empty = EmptyView::new();
        let stack = gtk::Stack::new();

        stack.add_named(&episodes.container, "episodes");
        stack.add_named(&empty.container, "empty");

        if episodes.is_empty() {
            stack.set_visible_child_name("empty");
        } else {
            stack.set_visible_child_name("episodes");
        }

        Rc::new(EpisodeStack {
            // empty,
            // populated: pop,
            stack,
        })
    }

    pub fn update(&self) {
        let old = self.stack.get_child_by_name("episodes").unwrap();
        let eps = EpisodesView::new();

        self.stack.remove(&old);
        self.stack.add_named(&eps.container, "episodes");

        if eps.is_empty() {
            self.stack.set_visible_child_name("empty");
        } else {
            self.stack.set_visible_child_name("episodes");
        }

        old.destroy();
    }
}

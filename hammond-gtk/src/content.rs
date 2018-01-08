use gtk;
use gtk::Cast;
use gtk::prelude::*;

use hammond_data::Podcast;
use hammond_data::dbqueries;

use views::shows::ShowsPopulated;
use views::empty::EmptyView;
use views::episodes::EpisodesView;

use widgets::show::ShowWidget;
use app::Action;

use std::sync::Arc;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Content {
    stack: gtk::Stack,
    shows: Arc<ShowStack>,
    episodes: Arc<EpisodeStack>,
    sender: Sender<Action>,
}

impl Content {
    pub fn new(sender: Sender<Action>) -> Arc<Content> {
        let stack = gtk::Stack::new();
        let episodes = EpisodeStack::new(sender.clone());
        let shows = ShowStack::new(sender.clone());

        stack.add_titled(&episodes.stack, "episodes", "Episodes");
        stack.add_titled(&shows.stack, "shows", "Shows");

        Arc::new(Content {
            stack,
            shows,
            episodes,
            sender,
        })
    }

    pub fn update(&self) {
        self.update_episode_view();
        self.update_shows_view();
        self.update_widget()
    }

    pub fn update_episode_view(&self) {
        self.episodes.update();
    }

    pub fn update_episode_view_if_baground(&self) {
        if self.stack.get_visible_child_name() != Some("episodes".into()) {
            self.episodes.update();
        }
    }

    pub fn update_shows_view(&self) {
        self.shows.update_podcasts();
    }

    pub fn update_widget(&self) {
        self.shows.update_widget();
    }

    pub fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub fn get_shows(&self) -> Arc<ShowStack> {
        self.shows.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ShowStack {
    stack: gtk::Stack,
    sender: Sender<Action>,
}

impl ShowStack {
    fn new(sender: Sender<Action>) -> Arc<ShowStack> {
        let stack = gtk::Stack::new();

        let show = Arc::new(ShowStack {
            stack,
            sender: sender.clone(),
        });

        let pop = ShowsPopulated::new(show.clone(), sender.clone());
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

    // pub fn update(&self) {
    //     self.update_widget();
    //     self.update_podcasts();
    // }

    pub fn update_podcasts(&self) {
        let vis = self.stack.get_visible_child_name().unwrap();

        let old = self.stack
            .get_child_by_name("podcasts")
            // This is guaranted to exists, based on `ShowStack::new()`.
            .unwrap()
            .downcast::<gtk::Box>()
            // This is guaranted to be a Box based on the `ShowsPopulated` impl.
            .unwrap();
        debug!("Name: {:?}", WidgetExt::get_name(&old));

        let scrolled_window = old.get_children()
            .first()
            // This is guaranted to exist based on the show_widget.ui file.
            .unwrap()
            .clone()
            .downcast::<gtk::ScrolledWindow>()
            // This is guaranted based on the show_widget.ui file.
            .unwrap();
        debug!("Name: {:?}", WidgetExt::get_name(&scrolled_window));

        let pop = ShowsPopulated::new(Arc::new(self.clone()), self.sender.clone());
        // Copy the vertical scrollbar adjustment from the old view into the new one.
        scrolled_window
            .get_vadjustment()
            .map(|x| pop.set_vadjustment(&x));

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
        let old = self.stack
            .get_child_by_name("widget")
            // This is guaranted to exists, based on `ShowStack::new()`.
            .unwrap()
            .downcast::<gtk::Box>()
            // This is guaranted to be a Box based on the `ShowWidget` impl.
            .unwrap();
        debug!("Name: {:?}", WidgetExt::get_name(&old));

        let scrolled_window = old.get_children()
            .first()
            // This is guaranted to exist based on the show_widget.ui file.
            .unwrap()
            .clone()
            .downcast::<gtk::ScrolledWindow>()
            // This is guaranted based on the show_widget.ui file.
            .unwrap();
        debug!("Name: {:?}", WidgetExt::get_name(&scrolled_window));

        let new = ShowWidget::new(Arc::new(self.clone()), pd, self.sender.clone());
        // Copy the vertical scrollbar adjustment from the old view into the new one.
        scrolled_window
            .get_vadjustment()
            .map(|x| new.set_vadjustment(&x));

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
    stack: gtk::Stack,
    sender: Sender<Action>,
}

impl EpisodeStack {
    fn new(sender: Sender<Action>) -> Arc<EpisodeStack> {
        let episodes = EpisodesView::new(sender.clone());
        let empty = EmptyView::new();
        let stack = gtk::Stack::new();

        stack.add_named(&episodes.container, "episodes");
        stack.add_named(&empty.container, "empty");

        if episodes.is_empty() {
            stack.set_visible_child_name("empty");
        } else {
            stack.set_visible_child_name("episodes");
        }

        Arc::new(EpisodeStack { stack, sender })
    }

    pub fn update(&self) {
        let old = self.stack
            .get_child_by_name("episodes")
            // This is guaranted to exists, based on `EpisodeStack::new()`.
            .unwrap()
            .downcast::<gtk::Box>()
            // This is guaranted to be a Box based on the `EpisodesView` impl.
            .unwrap();
        debug!("Name: {:?}", WidgetExt::get_name(&old));

        let scrolled_window = old.get_children()
            .first()
            // This is guaranted to exist based on the episodes_view.ui file.
            .unwrap()
            .clone()
            .downcast::<gtk::ScrolledWindow>()
            // This is guaranted based on the episodes_view.ui file.
            .unwrap();
        debug!("Name: {:?}", WidgetExt::get_name(&scrolled_window));

        let eps = EpisodesView::new(self.sender.clone());
        // Copy the vertical scrollbar adjustment from the old view into the new one.
        scrolled_window
            .get_vadjustment()
            .map(|x| eps.set_vadjustment(&x));

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

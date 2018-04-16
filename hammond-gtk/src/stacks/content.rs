use gtk;
use gtk::prelude::*;

use failure::Error;

use app::Action;
use stacks::EpisodeStack;
use stacks::ShowStack;

use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Content {
    stack: gtk::Stack,
    shows: Arc<ShowStack>,
    episodes: Arc<EpisodeStack>,
    sender: Sender<Action>,
}

impl Content {
    pub fn new(sender: Sender<Action>) -> Result<Content, Error> {
        let stack = gtk::Stack::new();
        let episodes = Arc::new(EpisodeStack::new(sender.clone())?);
        let shows = Arc::new(ShowStack::new(sender.clone())?);

        stack.add_titled(&episodes.get_stack(), "episodes", "Episodes");
        stack.add_titled(&shows.get_stack(), "shows", "Shows");

        Ok(Content {
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

    // TODO: Maybe propagate the error?
    pub fn update_episode_view(&self) {
        self.episodes
            .update()
            .map_err(|err| error!("Failed to update EpisodeView: {}", err))
            .ok();
    }

    pub fn update_episode_view_if_baground(&self) {
        if self.stack.get_visible_child_name() != Some("episodes".into()) {
            self.update_episode_view();
        }
    }

    pub fn update_shows_view(&self) {
        self.shows
            .update_podcasts()
            .map_err(|err| error!("Failed to update ShowsView: {}", err))
            .ok();
    }

    pub fn update_widget(&self) {
        self.shows
            .update_widget()
            .map_err(|err| error!("Failed to update ShowsWidget: {}", err))
            .ok();
    }

    pub fn update_widget_if_same(&self, pid: i32) {
        self.shows
            .update_widget_if_same(pid)
            .map_err(|err| error!("Failed to update ShowsWidget: {}", err))
            .ok();
    }

    pub fn update_widget_if_visible(&self) {
        if self.stack.get_visible_child_name() == Some("shows".to_string())
            && self.shows.get_stack().get_visible_child_name() == Some("widget".to_string())
        {
            self.update_widget();
        }
    }

    pub fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub fn get_shows(&self) -> Arc<ShowStack> {
        self.shows.clone()
    }
}

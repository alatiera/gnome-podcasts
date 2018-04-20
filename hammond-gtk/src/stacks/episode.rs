use gtk;
use gtk::prelude::*;

use failure::Error;
use hammond_data::dbqueries::is_episodes_populated;
use hammond_data::errors::DataError;

use app::Action;
use views::{EmptyView, EpisodesView};

use std::rc::Rc;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct EpisodeStack {
    stack: gtk::Stack,
    empty: EmptyView,
    episodes: Rc<EpisodesView>,
    sender: Sender<Action>,
}

impl EpisodeStack {
    pub fn new(sender: Sender<Action>) -> Result<EpisodeStack, Error> {
        let episodes = EpisodesView::new(sender.clone())?;
        let empty = EmptyView::new();
        let stack = gtk::Stack::new();

        stack.add_named(&episodes.container, "episodes");
        stack.add_named(&empty.container, "empty");
        set_stack_visible(&stack)?;

        Ok(EpisodeStack {
            stack,
            empty,
            episodes,
            sender,
        })
    }

    pub fn update(&mut self) -> Result<(), Error> {
        // Copy the vertical scrollbar adjustment from the old view.
        self.episodes
            .save_alignment()
            .map_err(|err| error!("Failed to set episodes_view allignment: {}", err))
            .ok();

        self.replace_view()?;
        set_stack_visible(&self.stack)?;
        Ok(())
    }

    fn replace_view(&mut self) -> Result<(), Error> {
        // Get the container of the view
        let old = self.episodes.container.clone();
        let eps = EpisodesView::new(self.sender.clone())?;

        // Remove the old widget and add the new one
        self.stack.remove(&old);
        self.stack.add_named(&eps.container, "episodes");

        // replace view in the struct too
        self.episodes = eps;

        // This might not be needed
        old.destroy();

        Ok(())
    }

    pub fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }
}

#[inline]
fn set_stack_visible(stack: &gtk::Stack) -> Result<(), DataError> {
    if is_episodes_populated()? {
        stack.set_visible_child_name("episodes");
    } else {
        stack.set_visible_child_name("empty");
    };

    Ok(())
}

use gtk;
use gtk::prelude::*;
use gtk::Cast;

use failure::Error;
use hammond_data::dbqueries::is_episodes_populated;
use hammond_data::errors::DataError;

use app::Action;
use views::{EmptyView, EpisodesView};

use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct EpisodeStack {
    stack: gtk::Stack,
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

        Ok(EpisodeStack { stack, sender })
    }

    // Look into refactoring to a state-machine.
    pub fn update(&self) -> Result<(), Error> {
        let old = self.stack
            .get_child_by_name("episodes")
            .ok_or_else(|| format_err!("Faild to get \"episodes\" child from the stack."))?
            .downcast::<gtk::Box>()
            .map_err(|_| format_err!("Failed to downcast stack child to a Box."))?;
        debug!("Name: {:?}", WidgetExt::get_name(&old));

        // let scrolled_window = old.get_children()
        //     .first()
        //     .ok_or_else(|| format_err!("Box container has no childs."))?
        //     .clone()
        //     .downcast::<gtk::ScrolledWindow>()
        //     .map_err(|_| format_err!("Failed to downcast stack child to a ScrolledWindow."))?;
        // debug!("Name: {:?}", WidgetExt::get_name(&scrolled_window));

        let eps = EpisodesView::new(self.sender.clone())?;
        // Copy the vertical scrollbar adjustment from the old view into the new one.
        // scrolled_window
        //     .get_vadjustment()
        //     .map(|x| eps.set_vadjustment(&x));

        self.stack.remove(&old);
        self.stack.add_named(&eps.container, "episodes");
        set_stack_visible(&self.stack)?;

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

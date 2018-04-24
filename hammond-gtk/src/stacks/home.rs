use gtk;
use gtk::prelude::*;

use failure::Error;
use hammond_data::dbqueries::is_episodes_populated;
use hammond_data::errors::DataError;

use app::Action;
use widgets::{EmptyView, HomeView};

use std::rc::Rc;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
enum State {
    Home,
    Empty,
}

#[derive(Debug, Clone)]
pub struct HomeStack {
    empty: EmptyView,
    episodes: Rc<HomeView>,
    stack: gtk::Stack,
    state: State,
    sender: Sender<Action>,
}

impl HomeStack {
    pub fn new(sender: Sender<Action>) -> Result<HomeStack, Error> {
        let episodes = HomeView::new(sender.clone())?;
        let empty = EmptyView::new();
        let stack = gtk::Stack::new();
        let state = State::Empty;

        stack.add_named(&episodes.container, "home");
        stack.add_named(&empty.container, "empty");

        let mut home = HomeStack {
            empty,
            episodes,
            stack,
            state,
            sender,
        };

        home.determine_state()?;
        Ok(home)
    }

    pub fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub fn update(&mut self) -> Result<(), Error> {
        // Copy the vertical scrollbar adjustment from the old view.
        self.episodes
            .save_alignment()
            .map_err(|err| error!("Failed to set episodes_view allignment: {}", err))
            .ok();

        self.replace_view()?;
        self.determine_state().map_err(From::from)
    }

    fn replace_view(&mut self) -> Result<(), Error> {
        // Get the container of the view
        let old = &self.episodes.container.clone();
        let eps = HomeView::new(self.sender.clone())?;

        // Remove the old widget and add the new one
        self.stack.remove(old);
        self.stack.add_named(&eps.container, "home");

        // replace view in the struct too
        self.episodes = eps;

        // This might not be needed
        old.destroy();

        Ok(())
    }

    #[inline]
    fn switch_visible(&mut self, s: State) {
        use self::State::*;

        match s {
            Home => {
                self.stack.set_visible_child_name("home");
                self.state = Home;
            }
            Empty => {
                self.stack.set_visible_child_name("empty");
                self.state = Empty;
            }
        }
    }

    #[inline]
    fn determine_state(&mut self) -> Result<(), DataError> {
        if is_episodes_populated()? {
            self.switch_visible(State::Home);
        } else {
            self.switch_visible(State::Empty);
        };

        Ok(())
    }
}

use gtk;
use gtk::prelude::*;

use crossbeam_channel::Sender;
use failure::Error;
use hammond_data::dbqueries::is_podcasts_populated;

use app::Action;
use stacks::PopulatedStack;
use utils::get_ignored_shows;
use widgets::EmptyView;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
pub enum ShowState {
    Populated,
    Empty,
}

#[derive(Debug, Clone)]
pub struct ShowStack {
    empty: EmptyView,
    populated: Rc<RefCell<PopulatedStack>>,
    stack: gtk::Stack,
    state: ShowState,
    sender: Sender<Action>,
}

impl ShowStack {
    pub fn new(sender: Sender<Action>) -> Result<Self, Error> {
        let populated = Rc::new(RefCell::new(PopulatedStack::new(sender.clone())?));
        let empty = EmptyView::new();
        let stack = gtk::Stack::new();
        let state = ShowState::Empty;

        stack.add_named(&populated.borrow().container(), "populated");
        stack.add_named(&empty.container, "empty");

        let mut show = ShowStack {
            empty,
            populated,
            stack,
            state,
            sender,
        };

        show.determine_state()?;
        Ok(show)
    }

    pub fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub fn populated(&self) -> Rc<RefCell<PopulatedStack>> {
        self.populated.clone()
    }

    pub fn update(&mut self) -> Result<(), Error> {
        self.populated.borrow_mut().update();
        self.determine_state()
    }

    fn switch_visible(&mut self, s: ShowState) {
        use self::ShowState::*;

        match s {
            Populated => {
                self.stack.set_visible_child_name("populated");
                self.state = Populated;
            }
            Empty => {
                self.stack.set_visible_child_name("empty");
                self.state = Empty;
            }
        };
    }

    fn determine_state(&mut self) -> Result<(), Error> {
        use self::ShowState::*;

        let ign = get_ignored_shows()?;
        debug!("IGNORED SHOWS {:?}", ign);
        if is_podcasts_populated(&ign)? {
            self.switch_visible(Populated);
        } else {
            self.switch_visible(Empty);
        };

        Ok(())
    }
}

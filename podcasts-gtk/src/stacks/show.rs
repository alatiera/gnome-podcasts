use gtk;
use gtk::prelude::*;

use crossbeam_channel::Sender;
use failure::Error;
use podcasts_data::dbqueries::is_podcasts_populated;

use app::Action;
use stacks::content::State;
use stacks::PopulatedStack;
use utils::get_ignored_shows;
use widgets::EmptyView;

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) struct ShowStack {
    empty: EmptyView,
    populated: Rc<RefCell<PopulatedStack>>,
    stack: gtk::Stack,
    state: State,
    sender: Sender<Action>,
}

impl ShowStack {
    pub(crate) fn new(sender: Sender<Action>) -> Self {
        let populated = Rc::new(RefCell::new(PopulatedStack::new(sender.clone())));
        let empty = EmptyView::default();
        let stack = gtk::Stack::new();
        let state = State::Empty;

        stack.add_named(&populated.borrow().container(), "populated");
        stack.add_named(empty.deref(), "empty");

        let mut show = ShowStack {
            empty,
            populated,
            stack,
            state,
            sender,
        };

        let res = show.determine_state();
        debug_assert!(res.is_ok());
        show
    }

    pub(crate) fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub(crate) fn populated(&self) -> Rc<RefCell<PopulatedStack>> {
        self.populated.clone()
    }

    pub(crate) fn update(&mut self) -> Result<(), Error> {
        self.populated.borrow_mut().update();
        self.determine_state()
    }

    pub(crate) fn switch_visible(&mut self, s: State) {
        use self::State::*;

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
        let ign = get_ignored_shows()?;
        debug!("IGNORED SHOWS {:?}", ign);
        if is_podcasts_populated(&ign)? {
            self.sender.send(Action::PopulatedState);
        } else {
            self.sender.send(Action::EmptyState);
        };

        Ok(())
    }
}

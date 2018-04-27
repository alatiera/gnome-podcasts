use gtk;
use gtk::prelude::*;

use failure::Error;

use app::Action;
use stacks::{HomeStack, ShowStack};

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Content {
    stack: gtk::Stack,
    shows: Rc<RefCell<ShowStack>>,
    home: Rc<RefCell<HomeStack>>,
    sender: Sender<Action>,
}

impl Content {
    pub fn new(sender: Sender<Action>) -> Result<Content, Error> {
        let stack = gtk::Stack::new();
        let home = Rc::new(RefCell::new(HomeStack::new(sender.clone())?));
        let shows = Rc::new(RefCell::new(ShowStack::new(sender.clone())?));

        stack.add_titled(&home.borrow().get_stack(), "home", "Recent");
        stack.add_titled(&shows.borrow().get_stack(), "shows", "Shows");

        Ok(Content {
            stack,
            shows,
            home,
            sender,
        })
    }

    pub fn update(&self) {
        self.update_home();
        self.update_shows();
    }

    pub fn update_home(&self) {
        self.home
            .borrow_mut()
            .update()
            .map_err(|err| error!("Failed to update HomeView: {}", err))
            .ok();
    }

    pub fn update_home_if_background(&self) {
        if self.stack.get_visible_child_name() != Some("home".into()) {
            self.update_home();
        }
    }

    fn update_shows(&self) {
        self.shows
            .borrow_mut()
            .update()
            .map_err(|err| error!("Failed to update ShowsView: {}", err))
            .ok();
    }

    pub fn update_shows_view(&self) {
        self.shows
            .borrow_mut()
            .update()
            .map_err(|err| error!("Failed to update ShowsView: {}", err))
            .ok();
    }

    pub fn update_widget_if_same(&self, pid: i32) {
        let pop = self.shows.borrow().populated();
        pop.borrow_mut()
            .update_widget_if_same(pid)
            .map_err(|err| error!("Failed to update ShowsWidget: {}", err))
            .ok();
    }

    pub fn get_stack(&self) -> gtk::Stack {
        self.stack.clone()
    }

    pub fn get_shows(&self) -> Rc<RefCell<ShowStack>> {
        self.shows.clone()
    }
}

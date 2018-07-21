use gtk;
use gtk::prelude::*;
use gtk::StackTransitionType;

use crossbeam_channel::Sender;
use failure::Error;

use hammond_data::dbqueries;
use hammond_data::Show;

use app::Action;
use widgets::{ShowWidget, ShowsView};

use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum PopulatedState {
    View,
    Widget,
}

#[derive(Debug, Clone)]
pub struct PopulatedStack {
    container: gtk::Box,
    populated: Rc<ShowsView>,
    show: Rc<ShowWidget>,
    stack: gtk::Stack,
    state: PopulatedState,
    sender: Sender<Action>,
}

impl PopulatedStack {
    pub fn new(sender: Sender<Action>) -> PopulatedStack {
        let stack = gtk::Stack::new();
        let state = PopulatedState::View;
        let populated = ShowsView::new(sender.clone());
        let show = Rc::new(ShowWidget::default());
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        stack.add_named(&populated.container, "shows");
        stack.add_named(&show.container, "widget");
        container.add(&stack);
        container.show_all();

        PopulatedStack {
            container,
            stack,
            populated,
            show,
            state,
            sender,
        }
    }

    pub fn update(&mut self) {
        self.update_widget().map_err(|err| format!("{}", err)).ok();
        self.update_shows().map_err(|err| format!("{}", err)).ok();
    }

    pub fn update_shows(&mut self) -> Result<(), Error> {
        // The current visible child might change depending on
        // removal and insertion in the gtk::Stack, so we have
        // to make sure it will stay the same.
        let s = self.state;
        self.replace_shows()?;
        self.switch_visible(s, StackTransitionType::Crossfade);

        Ok(())
    }

    pub fn replace_shows(&mut self) -> Result<(), Error> {
        let old = &self.populated.container.clone();
        debug!("Name: {:?}", WidgetExt::get_name(old));

        self.populated
            .save_alignment()
            .map_err(|err| error!("Failed to set episodes_view allignment: {}", err))
            .ok();

        let pop = ShowsView::new(self.sender.clone());
        self.populated = pop;
        self.stack.remove(old);
        self.stack.add_named(&self.populated.container, "shows");

        old.destroy();
        Ok(())
    }

    pub fn replace_widget(&mut self, pd: Arc<Show>) -> Result<(), Error> {
        let old = self.show.container.clone();

        // save the ShowWidget vertical scrollabar alignment
        self.show.show_id().map(|id| self.show.save_vadjustment(id));

        let new = ShowWidget::new(pd, self.sender.clone());
        self.show = new;
        self.stack.remove(&old);
        self.stack.add_named(&self.show.container, "widget");

        // The current visible child might change depending on
        // removal and insertion in the gtk::Stack, so we have
        // to make sure it will stay the same.
        let s = self.state;
        self.switch_visible(s, StackTransitionType::None);

        Ok(())
    }

    pub fn update_widget(&mut self) -> Result<(), Error> {
        let old = self.show.container.clone();
        let id = self.show.show_id();
        if id.is_none() {
            return Ok(());
        }

        let pd = dbqueries::get_podcast_from_id(id.unwrap_or_default())?;
        self.replace_widget(Arc::new(pd))?;

        // The current visible child might change depending on
        // removal and insertion in the gtk::Stack, so we have
        // to make sure it will stay the same.
        let s = self.state;
        self.switch_visible(s, StackTransitionType::Crossfade);

        old.destroy();
        Ok(())
    }

    // Only update widget if its show_id is equal to pid.
    pub fn update_widget_if_same(&mut self, pid: i32) -> Result<(), Error> {
        if self.show.show_id() != Some(pid) {
            debug!("Different widget. Early return");
            return Ok(());
        }

        self.update_widget()
    }

    pub fn container(&self) -> gtk::Box {
        self.container.clone()
    }

    pub fn switch_visible(&mut self, state: PopulatedState, animation: StackTransitionType) {
        use self::PopulatedState::*;

        match state {
            View => {
                self.stack.set_visible_child_full("shows", animation);
                self.state = View;
            }
            Widget => {
                self.stack.set_visible_child_full("widget", animation);
                self.state = Widget;
            }
        }
    }
}

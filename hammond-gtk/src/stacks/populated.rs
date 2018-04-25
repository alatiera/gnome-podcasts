use gtk;
use gtk::prelude::*;

use failure::Error;

use hammond_data::dbqueries;
use hammond_data::Podcast;

use app::Action;
use widgets::{ShowWidget, ShowsPopulated};

use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum PopulatedState {
    ShowsView,
    ShowWidget,
}

#[derive(Debug, Clone)]
pub struct PopulatedStack {
    container: gtk::Box,
    populated: Rc<ShowsPopulated>,
    show: Rc<ShowWidget>,
    stack: gtk::Stack,
    state: PopulatedState,
    sender: Sender<Action>,
}

impl PopulatedStack {
    pub fn new(sender: Sender<Action>) -> Result<PopulatedStack, Error> {
        let stack = gtk::Stack::new();
        let state = PopulatedState::ShowsView;
        let populated = ShowsPopulated::new(sender.clone())?;
        let show = Rc::new(ShowWidget::default());
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        stack.add_named(&populated.container, "shows");
        stack.add_named(&show.container, "widget");
        container.add(&stack);
        container.show_all();

        let show = PopulatedStack {
            container,
            stack,
            populated,
            show,
            state,
            sender,
        };

        Ok(show)
    }

    pub fn update(&mut self) {
        self.update_widget().map_err(|err| format!("{}", err)).ok();
        self.update_shows().map_err(|err| format!("{}", err)).ok();
    }

    pub fn update_shows(&mut self) -> Result<(), Error> {
        let old = &self.populated.container.clone();
        debug!("Name: {:?}", WidgetExt::get_name(old));

        let pop = ShowsPopulated::new(self.sender.clone())?;
        self.populated = pop;
        self.stack.remove(old);
        self.stack.add_named(&self.populated.container, "shows");

        // The current visible child might change depending on
        // removal and insertion in the gtk::Stack, so we have
        // to make sure it will stay the same.
        let s = self.state.clone();
        self.switch_visible(s);

        old.destroy();
        Ok(())
    }

    pub fn replace_widget(&mut self, pd: Arc<Podcast>) -> Result<(), Error> {
        let old = self.show.container.clone();

        // save the ShowWidget vertical scrollabar alignment
        self.show
            .podcast_id()
            .map(|id| self.show.save_vadjustment(id));

        let new = ShowWidget::new(pd, self.sender.clone());
        self.show = new;
        self.stack.remove(&old);
        self.stack.add_named(&self.show.container, "widget");

        // The current visible child might change depending on
        // removal and insertion in the gtk::Stack, so we have
        // to make sure it will stay the same.
        let s = self.state.clone();
        self.switch_visible(s);

        Ok(())
    }

    pub fn update_widget(&mut self) -> Result<(), Error> {
        let old = self.show.container.clone();
        let id = self.show.podcast_id();
        if id.is_none() {
            return Ok(());
        }

        let pd = dbqueries::get_podcast_from_id(id.unwrap_or_default())?;
        self.replace_widget(Arc::new(pd))?;

        // The current visible child might change depending on
        // removal and insertion in the gtk::Stack, so we have
        // to make sure it will stay the same.
        let s = self.state.clone();
        self.switch_visible(s);

        old.destroy();
        Ok(())
    }

    // Only update widget if its podcast_id is equal to pid.
    pub fn update_widget_if_same(&mut self, pid: i32) -> Result<(), Error> {
        if self.show.podcast_id() != Some(pid) {
            debug!("Different widget. Early return");
            return Ok(());
        }

        self.update_widget()
    }

    pub fn container(&self) -> gtk::Box {
        self.container.clone()
    }

    #[inline]
    pub fn switch_visible(&mut self, state: PopulatedState) {
        use self::PopulatedState::*;

        match state {
            ShowsView => {
                self.stack
                    .set_visible_child_full("shows", gtk::StackTransitionType::SlideRight);
                self.state = ShowsView;
            }
            ShowWidget => {
                self.stack
                    .set_visible_child_full("widget", gtk::StackTransitionType::SlideLeft);
                self.state = ShowWidget;
            }
        }
    }
}

use gtk;
use gtk::prelude::*;

use failure::Error;

use hammond_data::dbqueries;
use hammond_data::errors::DataError;
use hammond_data::Podcast;

use app::Action;
use widgets::{EmptyView, ShowWidget, ShowsPopulated};

use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ShowStack {
    stack: gtk::Stack,
    podcasts: Rc<ShowsPopulated>,
    show: Rc<ShowWidget>,
    empty: EmptyView,
    sender: Sender<Action>,
}

impl ShowStack {
    pub fn new(sender: Sender<Action>) -> Result<ShowStack, Error> {
        let stack = gtk::Stack::new();

        let podcasts = ShowsPopulated::new(sender.clone())?;
        let show = Rc::new(ShowWidget::default());
        let empty = EmptyView::new();

        stack.add_named(&podcasts.container, "podcasts");
        stack.add_named(&show.container, "widget");
        stack.add_named(&empty.container, "empty");
        set_stack_visible(&stack)?;

        let show = ShowStack {
            stack,
            podcasts,
            show,
            empty,
            sender,
        };

        Ok(show)
    }

    // pub fn update(&self) {
    //     self.update_widget();
    //     self.update_podcasts();
    // }

    pub fn update_podcasts(&mut self) -> Result<(), Error> {
        let vis = self.stack
            .get_visible_child_name()
            .ok_or_else(|| format_err!("Failed to get visible child name."))?;

        let old = &self.podcasts.container.clone();
        debug!("Name: {:?}", WidgetExt::get_name(old));

        let pop = ShowsPopulated::new(self.sender.clone())?;
        self.podcasts = pop;
        self.stack.remove(old);
        self.stack.add_named(&self.podcasts.container, "podcasts");

        if !dbqueries::is_podcasts_populated()? {
            self.stack.set_visible_child_name("empty");
        } else if vis != "empty" {
            self.stack.set_visible_child_name(&vis);
        } else {
            self.stack.set_visible_child_name("podcasts");
        }

        old.destroy();
        Ok(())
    }

    pub fn replace_widget(&mut self, pd: Arc<Podcast>) -> Result<(), Error> {
        let old = self.show.container.clone();

        let oldname = WidgetExt::get_name(&old);
        debug!("Name: {:?}", oldname);
        oldname
            .clone()
            .and_then(|id| id.parse().ok())
            .map(|id| self.show.save_vadjustment(id));

        let new = ShowWidget::new(pd, self.sender.clone());
        // Each composite ShowWidget is a gtkBox with the Podcast.id encoded in the
        // gtk::Widget name. It's a hack since we can't yet subclass GObject
        // easily.
        debug!(
            "Old widget Name: {:?}\nNew widget Name: {:?}",
            oldname,
            WidgetExt::get_name(&new.container)
        );

        self.show = new;
        self.stack.remove(&old);
        self.stack.add_named(&self.show.container, "widget");
        Ok(())
    }

    pub fn update_widget(&mut self) -> Result<(), Error> {
        let vis = self.stack
            .get_visible_child_name()
            .ok_or_else(|| format_err!("Failed to get visible child name."))?;

        let old = self.show.container.clone();
        let id = WidgetExt::get_name(&old);
        if id == Some("GtkBox".to_string()) || id.is_none() {
            return Ok(());
        }

        let id = id.ok_or_else(|| format_err!("Failed to get widget's name."))?;
        let pd = dbqueries::get_podcast_from_id(id.parse::<i32>()?)?;
        self.replace_widget(Arc::new(pd))?;
        self.stack.set_visible_child_name(&vis);
        old.destroy();
        Ok(())
    }

    // Only update widget if it's podcast_id is equal to pid.
    pub fn update_widget_if_same(&mut self, pid: i32) -> Result<(), Error> {
        let old = &self.show.container.clone();

        let id = WidgetExt::get_name(old);
        if id != Some(pid.to_string()) || id.is_none() {
            debug!("Different widget. Early return");
            return Ok(());
        }
        self.update_widget()
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

#[inline]
fn set_stack_visible(stack: &gtk::Stack) -> Result<(), DataError> {
    if dbqueries::is_podcasts_populated()? {
        stack.set_visible_child_name("podcasts")
    } else {
        stack.set_visible_child_name("empty")
    }

    Ok(())
}

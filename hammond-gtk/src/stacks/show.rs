use gtk;
use gtk::Cast;
use gtk::prelude::*;

use failure::Error;

use hammond_data::Podcast;
use hammond_data::dbqueries;

use views::{EmptyView, ShowsPopulated};

use app::Action;
use widgets::ShowWidget;

use std::sync::Arc;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct ShowStack {
    stack: gtk::Stack,
    sender: Sender<Action>,
}

impl ShowStack {
    pub fn new(sender: Sender<Action>) -> Result<ShowStack, Error> {
        let stack = gtk::Stack::new();

        let show = ShowStack {
            stack,
            sender: sender.clone(),
        };

        let pop = ShowsPopulated::new(sender.clone())?;
        let widget = ShowWidget::default();
        let empty = EmptyView::new();

        show.stack.add_named(&pop.container, "podcasts");
        show.stack.add_named(&widget.container, "widget");
        show.stack.add_named(&empty.container, "empty");

        if pop.is_empty() {
            show.stack.set_visible_child_name("empty")
        } else {
            show.stack.set_visible_child_name("podcasts")
        }

        Ok(show)
    }

    // pub fn update(&self) {
    //     self.update_widget();
    //     self.update_podcasts();
    // }

    pub fn update_podcasts(&self) -> Result<(), Error> {
        let vis = self.stack
            .get_visible_child_name()
            .ok_or_else(|| format_err!("Failed to get visible child name."))?;

        let old = self.stack
            .get_child_by_name("podcasts")
            .ok_or_else(|| format_err!("Faild to get \"podcasts\" child from the stack."))?
            .downcast::<gtk::Box>()
            .map_err(|_| format_err!("Failed to downcast stack child to a Box."))?;
        debug!("Name: {:?}", WidgetExt::get_name(&old));

        let scrolled_window = old.get_children()
            .first()
            .ok_or_else(|| format_err!("Box container has no childs."))?
            .clone()
            .downcast::<gtk::ScrolledWindow>()
            .map_err(|_| format_err!("Failed to downcast stack child to a ScrolledWindow."))?;
        debug!("Name: {:?}", WidgetExt::get_name(&scrolled_window));

        let pop = ShowsPopulated::new(self.sender.clone())?;
        // Copy the vertical scrollbar adjustment from the old view into the new one.
        scrolled_window
            .get_vadjustment()
            .map(|x| pop.set_vadjustment(&x));

        self.stack.remove(&old);
        self.stack.add_named(&pop.container, "podcasts");

        if pop.is_empty() {
            self.stack.set_visible_child_name("empty");
        } else if vis != "empty" {
            self.stack.set_visible_child_name(&vis);
        } else {
            self.stack.set_visible_child_name("podcasts");
        }

        old.destroy();
        Ok(())
    }

    pub fn replace_widget(&self, pd: Arc<Podcast>) -> Result<(), Error> {
        let old = self.stack
            .get_child_by_name("widget")
            .ok_or_else(|| format_err!("Faild to get \"widget\" child from the stack."))?
            .downcast::<gtk::Box>()
            .map_err(|_| format_err!("Failed to downcast stack child to a Box."))?;
        debug!("Name: {:?}", WidgetExt::get_name(&old));

        let new = ShowWidget::new(pd, self.sender.clone());
        // Each composite ShowWidget is a gtkBox with the Podcast.id encoded in the
        // gtk::Widget name. It's a hack since we can't yet subclass GObject
        // easily.
        let oldid = WidgetExt::get_name(&old);
        let newid = WidgetExt::get_name(&new.container);
        debug!("Old widget Name: {:?}\nNew widget Name: {:?}", oldid, newid);

        // Only copy the old scrollbar if both widget's represent the same podcast.
        if newid == oldid {
            let scrolled_window = old.get_children()
                .first()
                .ok_or_else(|| format_err!("Box container has no childs."))?
                .clone()
                .downcast::<gtk::ScrolledWindow>()
                .map_err(|_| format_err!("Failed to downcast stack child to a ScrolledWindow."))?;
            debug!("Name: {:?}", WidgetExt::get_name(&scrolled_window));

            // Copy the vertical scrollbar adjustment from the old view into the new one.
            scrolled_window
                .get_vadjustment()
                .map(|x| new.set_vadjustment(&x));
        }

        self.stack.remove(&old);
        self.stack.add_named(&new.container, "widget");
        Ok(())
    }

    pub fn update_widget(&self) -> Result<(), Error> {
        let vis = self.stack
            .get_visible_child_name()
            .ok_or_else(|| format_err!("Failed to get visible child name."))?;
        let old = self.stack
            .get_child_by_name("widget")
            .ok_or_else(|| format_err!("Faild to get \"widget\" child from the stack."))?;

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
    pub fn update_widget_if_same(&self, pid: i32) -> Result<(), Error> {
        let old = self.stack
            .get_child_by_name("widget")
            .ok_or_else(|| format_err!("Faild to get \"widget\" child from the stack."))?;

        let id = WidgetExt::get_name(&old);
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

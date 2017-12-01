use gtk;
use gtk::prelude::*;
// use gdk_pixbuf::Pixbuf;

// use diesel::Identifiable;

use widgets::podcast::PodcastWidget;
use views::podcasts::PopulatedView;
use views::empty::EmptyView;

#[derive(Debug)]
pub struct Content {
    pub stack: gtk::Stack,
    widget: PodcastWidget,
    podcasts: PopulatedView,
    empty: EmptyView,
}

// #[derive(Debug)]
// pub struct Content {
//     pub stack: gtk::Stack,
//     pub state: ContentState,
//     widget: PodcastWidget,
//     pub podcasts: PopulatedView,
//     empty: EmptyView,
// }

#[derive(Debug)]
#[allow(dead_code)]
// TODO: find a way to wrap gtk::Stack into a State machine.
pub enum ContentState {
    Widget(PodcastWidget),
    Empty(EmptyView),
    Populated(PopulatedView),
}

impl Content {
    pub fn new() -> Content {
        let stack = gtk::Stack::new();
        let widget = PodcastWidget::new();
        let pop = PopulatedView::new();
        let empty = EmptyView::new();

        let content = Content {
            stack,
            widget,
            empty,
            podcasts: pop,
        };

        content.init();
        content
    }

    fn setup_stack(&self) {
        self.stack.add_named(&self.widget.container, "widget");
        self.stack.add_named(&self.podcasts.container, "podcasts");
        self.stack.add_named(&self.empty.container, "empty");

        self.stack.set_visible_child_name("podcasts")
    }

    fn init(&self) {
        self.setup_stack();
        self.podcasts.init(&self.stack);
        if self.podcasts.flowbox.get_children().is_empty() {
            self.stack.set_visible_child_name("empty");
        }
    }
}

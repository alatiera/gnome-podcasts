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
    // widget: PodcastWidget,
    // podcasts: PopulatedView,
    // empty: EmptyView,
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

        let content = Content {
            stack,
            // widget,
            // empty,
            // podcasts: pop,
        };

        content.init();
        content
    }

    fn init(&self) {
        let widget = PodcastWidget::new();
        let podcasts = PopulatedView::new();
        let empty = EmptyView::new();

        self.stack.add_named(&widget.container, "widget");
        self.stack.add_named(&podcasts.container, "podcasts");
        self.stack.add_named(&empty.container, "empty");
        self.stack.set_visible_child_name("podcasts");

        podcasts.init(&self.stack);
        if podcasts.flowbox.get_children().is_empty() {
            self.stack.set_visible_child_name("empty");
        }
    }
}

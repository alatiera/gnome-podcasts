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
    pub state: ContentState,
    widget: PodcastWidget,
    pub podcasts: PopulatedView,
    empty: EmptyView,
}

#[derive(Debug)]
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
        // TODO: Avoid cloning
        let state = ContentState::Populated(pop.clone());

        let content = Content {
            stack,
            state,
            widget,
            empty,
            podcasts: pop,
        };

        content.init();
        content
    }

    fn setup_stack(&self) {
        self.stack
            .set_transition_type(gtk::StackTransitionType::SlideLeftRight);

        self.stack.add_named(&self.widget.container, "pdw"); // Rename into "widget"
        self.stack.add_named(&self.podcasts.container, "fb_parent"); // Rename into "podcasts"
        self.stack.add_named(&self.empty.container, "empty"); // Rename into "empty"

        // FIXME: needs actuall logic
        self.stack.set_visible_child_name("fb_parent")
    }

    fn init(&self) {
        self.setup_stack();
        self.podcasts.init(&self.stack);
    }
}

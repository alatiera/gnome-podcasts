use gtk;
use gtk::prelude::*;

use hammond_data::Podcast;

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

fn replace_widget(stack: &gtk::Stack, pdw: &PodcastWidget) {
    let old = stack.get_child_by_name("widget").unwrap();
    stack.remove(&old);
    stack.add_named(&pdw.container, "widget");
    old.destroy();
}

fn replace_podcasts(stack: &gtk::Stack, pop: &PopulatedView) {
    let old = stack.get_child_by_name("podcasts").unwrap();
    stack.remove(&old);
    stack.add_named(&pop.container, "podcasts");
    old.destroy();
}

// This won't ever be needed probably
// pub fn replace_empty(stack: &gtk::Stack, emp: &EmptyView ) {
//     let old = stack.get_child_by_name("empty").unwrap();
//     stack.remove(&old);
//     stack.add_named(&emp.container, "empty");
//     old.destroy();
// }

#[allow(dead_code)]
pub fn show_widget(stack: &gtk::Stack) {
    stack.set_visible_child_name("widget")
}

pub fn show_podcasts(stack: &gtk::Stack) {
    stack.set_visible_child_name("podcasts")
}

pub fn show_empty(stack: &gtk::Stack) {
    stack.set_visible_child_name("empty")
}

pub fn update_podcasts(stack: &gtk::Stack) {
    let pods = PopulatedView::new_initialized(stack);

    if pods.flowbox.get_children().is_empty() {
        show_empty(stack)
    }

    replace_podcasts(stack, &pods);
}

pub fn update_widget(stack: &gtk::Stack, pd: &Podcast) {
    let pdw = PodcastWidget::new_initialized(stack, pd);
    replace_widget(stack, &pdw);
}

pub fn update_podcasts_preserve_vis(stack: &gtk::Stack) {
    let vis = stack.get_visible_child_name().unwrap();
    update_podcasts(stack);
    if vis != "empty" {
        stack.set_visible_child_name(&vis)
    }
}

pub fn update_widget_preserve_vis(stack: &gtk::Stack, pd: &Podcast) {
    let vis = stack.get_visible_child_name().unwrap();
    update_widget(stack, pd);
    stack.set_visible_child_name(&vis)
}

pub fn on_podcasts_child_activate(stack: &gtk::Stack, pd: &Podcast) {
    update_widget(stack, pd);
    stack.set_visible_child_full("widget", gtk::StackTransitionType::SlideLeft);
}

pub fn on_home_button_activate(stack: &gtk::Stack) {
    let vis = stack.get_visible_child_name().unwrap();

    if vis != "widget" {
        update_podcasts(stack);
    }

    show_podcasts(stack);
}

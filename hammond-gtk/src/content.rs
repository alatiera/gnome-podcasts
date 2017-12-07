use gtk;
use gtk::prelude::*;

use hammond_data::Podcast;

use widgets::podcast::PodcastWidget;
use views::podcasts::PopulatedView;
use views::empty::EmptyView;

#[derive(Debug)]
pub struct Content {
    pub stack: gtk::Stack,
    pub widget: PodcastWidget,
    pub podcasts: PopulatedView,
    pub empty: EmptyView,
}

impl Content {
    fn new() -> Content {
        let stack = gtk::Stack::new();

        let widget = PodcastWidget::new();
        let podcasts = PopulatedView::new();
        let empty = EmptyView::new();

        stack.add_named(&widget.container, "widget");
        stack.add_named(&podcasts.container, "podcasts");
        stack.add_named(&empty.container, "empty");

        Content {
            stack,
            widget,
            empty,
            podcasts,
        }
    }
}

#[derive(Debug)]
struct Empty {
    content: Content 
}

#[derive(Debug)]
struct PodcastsView {
    content: Content
}

#[derive(Debug)]
struct WidgetsView {
    content: Content
}

impl Empty {
    fn into_podcasts(self) -> Result<PodcastsView, Empty> {
        if self.content.podcasts.flowbox.get_children().is_empty() {
            return Err(self);
        }

        self.content.stack.set_visible_child_name("podcasts");

        Ok(
            PodcastsView {
                content: self.content
            }
        )
    }
}

impl PodcastsView {
    fn into_empty(self) -> Result<Empty, PodcastsView> {
        if !self.content.podcasts.flowbox.get_children().is_empty() {
            return Err(self);
        }

        self.content.stack.set_visible_child_name("empty");

        Ok(
            Empty {
                content: self.content
            }
        )
    }

    fn into_widget(self) -> WidgetsView {
        self.content.stack.set_visible_child_name("widget");

        WidgetsView {
            content: self.content
        }
    }
}

impl WidgetsView {
    fn into_podcasts(self) -> PodcastsView {
        self.content.stack.set_visible_child_name("podcasts");
        PodcastsView {
            content: self.content
        }
    }

    fn into_empty(self) -> Empty {
        self.content.stack.set_visible_child_name("empty");
        Empty {
            content: self.content
        }
    }
}

#[derive(Debug)]
pub enum ContentState {
    empty(Empty),
    pop(PodcastsView),
    pd(WidgetsView),
}

impl ContentState {
    pub fn new() -> ContentState {
        let content = Content::new();

        content.podcasts.init(&content.stack);
        if content.podcasts.flowbox.get_children().is_empty() {
            content.stack.set_visible_child_name("empty");
            return ContentState::empty(Empty { content })
        }

        content.stack.set_visible_child_name("podcasts");
        ContentState::pop(PodcastsView{ content })
    }

    pub fn get_stack(&self) -> gtk::Stack {
        match *self {
            ContentState::empty(ref e) => e.content.stack.clone(),
            ContentState::pop(ref e) => e.content.stack.clone(),
            ContentState::pd(ref e) => e.content.stack.clone(),
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

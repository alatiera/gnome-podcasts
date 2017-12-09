use gtk;
use gtk::prelude::*;

use hammond_data::Podcast;
use hammond_data::dbqueries;

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
    pub fn new() -> Content {
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

    pub fn new_initialized() -> Content {
        let ct = Content::new();
        ct.init();
        ct
    }

    pub fn init(&self) {
        self.podcasts.init(&self.stack);
        if self.podcasts.flowbox.get_children().is_empty() {
            self.stack.set_visible_child_name("empty");
            return;
        }

        self.stack.set_visible_child_name("podcasts");
    }

    fn replace_widget(&mut self, pdw: PodcastWidget) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("widget").unwrap();
        self.stack.remove(&old);

        self.widget = pdw;
        self.stack.add_named(&self.widget.container, "widget");
        self.stack.set_visible_child_name(&vis);
        old.destroy();
    }

    fn replace_podcasts(&mut self, pop: PopulatedView) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("podcasts").unwrap();
        self.stack.remove(&old);

        self.podcasts = pop;
        self.stack.add_named(&self.podcasts.container, "podcasts");
        self.stack.set_visible_child_name(&vis);
        old.destroy();
    }
}

#[derive(Debug)]
// Experiementing with Wrapping gtk::Stack into a State machine.
// Gonna revist it when TryInto trais is stabilized.
pub struct ContentState<S> {
    content: Content,
    state: S,
}

pub trait UpdateView {
    fn update(&mut self);
}

#[derive(Debug)]
pub struct Empty {}

#[derive(Debug)]
pub struct PodcastsView {}

#[derive(Debug)]
pub struct WidgetsView {}

impl Into<ContentState<PodcastsView>> for ContentState<Empty> {
    fn into(self) -> ContentState<PodcastsView> {
        self.content.stack.set_visible_child_name("podcasts");

        ContentState {
            content: self.content,
            state: PodcastsView {},
        }
    }
}

impl UpdateView for ContentState<Empty> {
    fn update(&mut self) {}
}

impl Into<ContentState<Empty>> for ContentState<PodcastsView> {
    fn into(self) -> ContentState<Empty> {
        self.content.stack.set_visible_child_name("empty");
        ContentState {
            content: self.content,
            state: Empty {},
        }
    }
}

impl Into<ContentState<WidgetsView>> for ContentState<PodcastsView> {
    fn into(self) -> ContentState<WidgetsView> {
        self.content.stack.set_visible_child_name("widget");

        ContentState {
            content: self.content,
            state: WidgetsView {},
        }
    }
}

impl UpdateView for ContentState<PodcastsView> {
    fn update(&mut self) {
        let pop = PopulatedView::new_initialized(&self.content.stack);
        self.content.replace_podcasts(pop)
    }
}

impl Into<ContentState<PodcastsView>> for ContentState<WidgetsView> {
    fn into(self) -> ContentState<PodcastsView> {
        self.content.stack.set_visible_child_name("podcasts");
        ContentState {
            content: self.content,
            state: PodcastsView {},
        }
    }
}

impl Into<ContentState<Empty>> for ContentState<WidgetsView> {
    fn into(self) -> ContentState<Empty> {
        self.content.stack.set_visible_child_name("empty");
        ContentState {
            content: self.content,
            state: Empty {},
        }
    }
}

impl UpdateView for ContentState<WidgetsView> {
    fn update(&mut self) {
        let old = self.content.stack.get_child_by_name("widget").unwrap();
        let id = WidgetExt::get_name(&old).unwrap();
        let pd = dbqueries::get_podcast_from_id(id.parse::<i32>().unwrap()).unwrap();

        let pdw = PodcastWidget::new_initialized(&self.content.stack, &pd);
        self.content.replace_widget(pdw);
    }
}

impl ContentState<PodcastsView> {
    #[allow(dead_code)]
    pub fn new() -> Result<ContentState<PodcastsView>, ContentState<Empty>> {
        let content = Content::new();

        content.podcasts.init(&content.stack);
        if content.podcasts.flowbox.get_children().is_empty() {
            content.stack.set_visible_child_name("empty");
            return Err(ContentState {
                content,
                state: Empty {},
            });
        }

        content.stack.set_visible_child_name("podcasts");
        Ok(ContentState {
            content,
            state: PodcastsView {},
        })
    }

    #[allow(dead_code)]
    pub fn get_stack(&self) -> gtk::Stack {
        self.content.stack.clone()
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

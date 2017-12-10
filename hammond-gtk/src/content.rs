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

        stack.add_titled(&widget.container, "widget", "Episodes");
        stack.add_titled(&podcasts.container, "podcasts", "Shows");
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
        self.stack
            .add_titled(&self.widget.container, "widget", "Episodes");
        self.stack.set_visible_child_name(&vis);
        old.destroy();
    }

    fn replace_podcasts(&mut self, pop: PopulatedView) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("podcasts").unwrap();
        self.stack.remove(&old);

        self.podcasts = pop;
        self.stack
            .add_titled(&self.podcasts.container, "podcasts", "Shows");
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

pub struct Empty;

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
    stack.add_titled(&pdw.container, "widget", "Episode");
    old.destroy();
}

fn replace_podcasts(stack: &gtk::Stack, pop: &PopulatedView) {
    let old = stack.get_child_by_name("podcasts").unwrap();
    stack.remove(&old);
    stack.add_titled(&pop.container, "podcasts", "Shows");
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

// FIXME: Rename and remove aliases
type ShowsPopulated = PopulatedView;
type ShowsEmpty = EmptyView;
type EpisodesPopulated = PodcastWidget;
type EpisodesEmpty = EmptyView;

struct Populated;
// struct Empty;
// struct Shows;
// struct Episodes;

// Thats probably too overengineered
// struct StackStateMachine<S, T> {
//     shows: ShowsMachine<S>,
//     episodes: EpisodesMachine<S>,
//     stack: gtk::Stack,
//     state: T,
// }

#[derive(Debug, Clone)]
struct ShowsMachine<S> {
    populated: ShowsPopulated,
    empty: ShowsEmpty,
    stack: gtk::Stack,
    state: S,
}

#[derive(Debug)]
struct EpisodesMachine<S> {
    populated: EpisodesPopulated,
    empty: EpisodesEmpty,
    stack: gtk::Stack,
    state: S,
}

// impl Into<StackStateMachine<Populated, Shows>> for StackStateMachine<Populated, Episodes> {
//     fn into(self) -> StackStateMachine<Populated, Shows> {
//         self.stack.set_visible_child_name("shows");

//         StackStateMachine {
//             shows: self.shows,
//             episodes: self.episodes,
//             stack: self.stack,
//             state: Shows {},
//         }
//     }
// }

// impl Into<StackStateMachine<Populated, Episodes>> for StackStateMachine<Populated, Shows> {
//     fn into(self) -> StackStateMachine<Populated, Episodes> {
//         self.stack.set_visible_child_name("episodes");

//         StackStateMachine {
//             shows: self.shows,
//             episodes: self.episodes,
//             stack: self.stack,
//             state: Episodes {},
//         }
//     }
// }

impl Into<ShowsMachine<Populated>> for ShowsMachine<Empty> {
    fn into(self) -> ShowsMachine<Populated> {
        self.stack.set_visible_child_name("populated");

        ShowsMachine {
            populated: self.populated,
            empty: self.empty,
            stack: self.stack,
            state: Populated {},
        }
    }
}

impl Into<ShowsMachine<Empty>> for ShowsMachine<Populated> {
    fn into(self) -> ShowsMachine<Empty> {
        self.stack.set_visible_child_name("empty");

        ShowsMachine {
            populated: self.populated,
            empty: self.empty,
            stack: self.stack,
            state: Empty {},
        }
    }
}

impl Into<EpisodesMachine<Populated>> for EpisodesMachine<Empty> {
    fn into(self) -> EpisodesMachine<Populated> {
        self.stack.set_visible_child_name("populated");

        EpisodesMachine {
            populated: self.populated,
            empty: self.empty,
            stack: self.stack,
            state: Populated {},
        }
    }
}

impl Into<EpisodesMachine<Empty>> for EpisodesMachine<Populated> {
    fn into(self) -> EpisodesMachine<Empty> {
        self.stack.set_visible_child_name("empty");

        EpisodesMachine {
            populated: self.populated,
            empty: self.empty,
            stack: self.stack,
            state: Empty {},
        }
    }
}

// enum StackStateWrapper<S> {
//     Shows(StackStateMachine<S, Shows>),
//     Episodes(StackStateMachine<S, Episodes>),
// }

enum ShowStateWrapper {
    Populated(EpisodesMachine<Populated>),
    Empty(EpisodesMachine<Empty>),
}

enum EpisodeStateWrapper {
    Populated(EpisodesMachine<Populated>),
    Empty(EpisodesMachine<Empty>),
}

// impl <S>StackStateWrapper<S> {
//     fn switch(mut self) -> Self {
//         match self {
//             StackStateWrapper::Shows(val) => StackStateWrapper::Episodes(val.into()),
//             StackStateWrapper::Episodes(val) => StackStateWrapper::Shows(val.into())
//         }
//     }
// }

impl ShowStateWrapper {
    fn switch(self) -> Self {
        match self {
            ShowStateWrapper::Populated(val) => ShowStateWrapper::Empty(val.into()),
            ShowStateWrapper::Empty(val) => ShowStateWrapper::Populated(val.into()),
        }
    }
}

impl EpisodeStateWrapper {
    fn switch(self) -> Self {
        match self {
            EpisodeStateWrapper::Populated(val) => EpisodeStateWrapper::Empty(val.into()),
            EpisodeStateWrapper::Empty(val) => EpisodeStateWrapper::Populated(val.into()),
        }
    }
}

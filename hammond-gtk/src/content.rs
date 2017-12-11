use gtk;
use gtk::prelude::*;

use hammond_data::Podcast;
use hammond_data::dbqueries;

use widgets::podcast::PodcastWidget;
use views::podcasts::PopulatedView;
use views::empty::EmptyView;

pub struct Content {
    pub stack: gtk::Stack,
    shows: ShowStateWrapper,
    episodes: EpisodeStateWrapper,
}

impl Content {
    pub fn new() -> Content {
        let stack = gtk::Stack::new();
        let shows = ShowStateWrapper::new();
        let episodes = EpisodeStateWrapper::new();

        let shows_stack = shows.get_stack();
        let ep_stack = episodes.get_stack();

        stack.add_titled(&ep_stack, "episodes", "Episodes");
        stack.add_titled(&shows_stack, "shows", "Shows");

        Content {
            stack,
            shows,
            episodes,
        }
    }

    // pub fn new_initialized() -> Content {
    //     let ct = Content::new();
    //     ct.init();
    //     ct
    // }

    // pub fn init(&self) {
    //     self.podcasts.init();
    //     if self.podcasts.flowbox.get_children().is_empty() {
    //         self.stack.set_visible_child_name("empty");
    //         return;
    //     }

    //     self.stack.set_visible_child_name("podcasts");
    // }

    // fn replace_widget(&mut self, pdw: PodcastWidget) {
    //     let vis = self.stack.get_visible_child_name().unwrap();
    //     let old = self.stack.get_child_by_name("widget").unwrap();
    //     self.stack.remove(&old);

    //     self.widget = pdw;
    //     self.stack
    //         .add_titled(&self.widget.container, "widget", "Episodes");
    //     self.stack.set_visible_child_name(&vis);
    //     old.destroy();
    // }
}

pub struct ContentState<S> {
    content: Content,
    state: S,
}

pub trait UpdateView {
    fn update(&mut self);
}

#[derive(Debug)]
pub struct PodcastsView {}

#[derive(Debug)]
pub struct WidgetsView {}

impl UpdateView for ContentState<WidgetsView> {
    fn update(&mut self) {
        let old = self.content.stack.get_child_by_name("widget").unwrap();
        let id = WidgetExt::get_name(&old).unwrap();
        let pd = dbqueries::get_podcast_from_id(id.parse::<i32>().unwrap()).unwrap();

        let pdw = PodcastWidget::new_initialized(&self.content.stack, &pd);
        // self.content.replace_widget(pdw);
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

pub fn update_podcasts(stack: &gtk::Stack) {
    let pods = PopulatedView::new_initialized();

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

// pub fn on_podcasts_child_activate(stack: &gtk::Stack, pd: &Podcast) {
//     update_widget(stack, pd);
//     stack.set_visible_child_full("widget", gtk::StackTransitionType::SlideLeft);
// }

// FIXME: Rename and remove aliases
type ShowsPopulated = PopulatedView;
type ShowsEmpty = EmptyView;
type EpisodesPopulated = PodcastWidget;
type EpisodesEmpty = EmptyView;

struct Populated;
struct Empty;
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

impl<S> ShowsMachine<S> {
    fn update(&mut self) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("shows").unwrap();
        self.stack.remove(&old);

        let pop = ShowsPopulated::new_initialized();
        self.populated = pop;
        self.stack
            .add_titled(&self.populated.container, "shows", "Shows");
        self.stack.set_visible_child_name(&vis);
    }
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

// TODO: Impl <From> instead of <Into>
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
    Populated(ShowsMachine<Populated>),
    Empty(ShowsMachine<Empty>),
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
    fn new() -> Self {
        let stack = gtk::Stack::new();
        let pop = ShowsPopulated::new_initialized();
        let empty = EmptyView::new();
        stack.add_named(&pop.container, "populated");
        stack.add_named(&empty.container, "empty");

        if pop.flowbox.get_children().is_empty() {
            stack.set_visible_child_name("empty");
            ShowStateWrapper::Empty(ShowsMachine {
                empty,
                populated: pop,
                stack,
                state: Empty {},
            })
        } else {
            stack.set_visible_child_name("populated");

            ShowStateWrapper::Populated(ShowsMachine {
                empty,
                populated: pop,
                stack,
                state: Populated {},
            })
        }
    }

    fn switch(self) -> Self {
        match self {
            ShowStateWrapper::Populated(val) => ShowStateWrapper::Empty(val.into()),
            ShowStateWrapper::Empty(val) => ShowStateWrapper::Populated(val.into()),
        }
    }

    fn update(&mut self) {
        match *self {
            ShowStateWrapper::Populated(ref mut val) => val.update(),
            ShowStateWrapper::Empty(ref mut val) => val.update(),
        }
    }

    fn get_stack(&self) -> gtk::Stack {
        match *self {
            ShowStateWrapper::Populated(ref val) => val.stack.clone(),
            ShowStateWrapper::Empty(ref val) => val.stack.clone(),
        }
    }
}

impl EpisodeStateWrapper {
    // FIXME:
    fn new() -> Self {
        let pop = PodcastWidget::new();
        let empty = EmptyView::new();
        let stack = gtk::Stack::new();

        stack.add_named(&pop.container, "populated");
        stack.add_named(&empty.container, "empty");
        stack.set_visible_child_name("empty");

        EpisodeStateWrapper::Empty(EpisodesMachine {
            empty,
            populated: pop,
            stack,
            state: Empty {},
        })
    }

    fn switch(self) -> Self {
        match self {
            EpisodeStateWrapper::Populated(val) => EpisodeStateWrapper::Empty(val.into()),
            EpisodeStateWrapper::Empty(val) => EpisodeStateWrapper::Populated(val.into()),
        }
    }

    fn get_stack(&self) -> gtk::Stack {
        match *self {
            EpisodeStateWrapper::Populated(ref val) => val.stack.clone(),
            EpisodeStateWrapper::Empty(ref val) => val.stack.clone(),
        }
    }
}

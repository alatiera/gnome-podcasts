use gtk;
use gtk::prelude::*;

// use hammond_data::Podcast;
use hammond_data::dbqueries;

use widgets::podcast::PodcastWidget;
use views::podcasts::PopulatedView;
use views::empty::EmptyView;

#[derive(Debug, Clone)]
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

    pub fn update(&mut self) {
        self.shows = self.shows.clone().update();
        // FIXME: like above
        self.episodes.update();
    }
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

#[derive(Debug, Clone)]
struct Populated;
#[derive(Debug, Clone)]
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
    fn new(state: S) -> ShowsMachine<S> {
        let stack = gtk::Stack::new();
        let pop = ShowsPopulated::new_initialized();
        let empty = EmptyView::new();
        stack.add_named(&pop.container, "populated");
        stack.add_named(&empty.container, "empty");

        ShowsMachine {
            empty,
            populated: pop,
            stack,
            state,
        }
    }

    fn update(&mut self) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("populated").unwrap();
        self.stack.remove(&old);

        let pop = ShowsPopulated::new_initialized();
        self.populated = pop;
        self.stack.add_named(&self.populated.container, "populated");
        self.stack.set_visible_child_name(&vis);
    }
}

#[derive(Debug, Clone)]
struct EpisodesMachine<S> {
    populated: EpisodesPopulated,
    empty: EpisodesEmpty,
    stack: gtk::Stack,
    state: S,
}

impl<S> EpisodesMachine<S> {
    // FIXME:
    fn update(&mut self) {
        let vis = self.stack.get_visible_child_name().unwrap();
        let old = self.stack.get_child_by_name("populated").unwrap();

        let id = WidgetExt::get_name(&old).unwrap();
        if id == "GtkBox" {
            return;
        }
        let pd = dbqueries::get_podcast_from_id(id.parse::<i32>().unwrap()).unwrap();
        let pdw = EpisodesPopulated::new_initialized(&self.stack, &pd);

        self.populated = pdw;
        self.stack.remove(&old);
        self.stack.add_named(&self.populated.container, "populated");
        self.stack.set_visible_child_name(&vis);
    }
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

#[derive(Debug, Clone)]
enum ShowStateWrapper {
    Populated(ShowsMachine<Populated>),
    Empty(ShowsMachine<Empty>),
}

#[derive(Debug, Clone)]
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
        let machine = ShowsMachine::new(Populated {});

        if machine.populated.flowbox.get_children().is_empty() {
            machine.stack.set_visible_child_name("empty");
            ShowStateWrapper::Empty(machine.into())
        } else {
            machine.stack.set_visible_child_name("populated");
            ShowStateWrapper::Populated(machine)
        }
    }

    fn update(mut self) -> Self {
        match self {
            ShowStateWrapper::Populated(ref mut val) => val.update(),
            ShowStateWrapper::Empty(ref mut val) => val.update(),
        }

        if self.is_empty() {
            match self {
                ShowStateWrapper::Populated(val) => ShowStateWrapper::Empty(val.into()),
                _ => self,
            }
        } else {
            match self {
                ShowStateWrapper::Empty(val) => ShowStateWrapper::Populated(val.into()),
                _ => self,
            }
        }
    }

    fn get_stack(&self) -> gtk::Stack {
        match *self {
            ShowStateWrapper::Populated(ref val) => val.stack.clone(),
            ShowStateWrapper::Empty(ref val) => val.stack.clone(),
        }
    }

    fn is_empty(&self) -> bool {
        match *self {
            ShowStateWrapper::Populated(ref val) => val.populated.flowbox.get_children().is_empty(),
            ShowStateWrapper::Empty(ref val) => val.populated.flowbox.get_children().is_empty(),
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

    fn update(&mut self) {
        match *self {
            EpisodeStateWrapper::Populated(ref mut val) => val.update(),
            EpisodeStateWrapper::Empty(ref mut val) => val.update(),
        }
    }

    fn get_stack(&self) -> gtk::Stack {
        match *self {
            EpisodeStateWrapper::Populated(ref val) => val.stack.clone(),
            EpisodeStateWrapper::Empty(ref val) => val.stack.clone(),
        }
    }
}

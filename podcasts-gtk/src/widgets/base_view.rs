use gtk::{self, prelude::*, Orientation, PolicyType};

#[derive(Debug, Clone)]
pub(crate) struct BaseView {
    container: gtk::Box,
    scrolled_window: gtk::ScrolledWindow,
}

impl Default for BaseView {
    fn default() -> Self {
        let container = gtk::Box::new(Orientation::Horizontal, 0);
        let scrolled_window = gtk::ScrolledWindow::new(None, None);

        scrolled_window.set_policy(PolicyType::Never, PolicyType::Automatic);
        container.add(&scrolled_window);
        container.show_all();

        BaseView {
            container,
            scrolled_window,
        }
    }
}

impl BaseView {
    pub(crate) fn container(&self) -> &gtk::Box {
        &self.container
    }

    pub(crate) fn scrolled_window(&self) -> &gtk::ScrolledWindow {
        &self.scrolled_window
    }

    pub(crate) fn add<T: IsA<gtk::Widget>>(&self, widget: &T) {
        self.scrolled_window.add(widget);
    }
}

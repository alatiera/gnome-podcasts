use gtk;
use std::ops::Deref;

#[derive(Clone, Debug)]
pub(crate) struct EmptyView(gtk::Box);

impl Deref for EmptyView {
    type Target = gtk::Box;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for EmptyView {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/empty_view.ui");
        let view: gtk::Box = builder.get_object("empty_view").unwrap();
        EmptyView(view)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EmptyShow(gtk::Box);

impl Deref for EmptyShow {
    type Target = gtk::Box;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for EmptyShow {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/empty_view.ui");
        let box_: gtk::Box = builder.get_object("empty_show").unwrap();
        EmptyShow(box_)
    }
}

use gtk;

#[derive(Debug, Clone)]
pub(crate) struct EmptyView {
    pub(crate) container: gtk::Box,
}

impl Default for EmptyView {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Podcasts/gtk/empty_view.ui");
        let view: gtk::Box = builder.get_object("empty_view").unwrap();

        EmptyView { container: view }
    }
}

impl EmptyView {
    pub(crate) fn new() -> EmptyView {
        EmptyView::default()
    }
}

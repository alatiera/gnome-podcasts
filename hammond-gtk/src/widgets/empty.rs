use gtk;

#[derive(Debug, Clone)]
pub struct EmptyView {
    pub container: gtk::Box,
}

impl Default for EmptyView {
    #[inline]
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/empty_view.ui");
        let view: gtk::Box = builder.get_object("empty_view").unwrap();

        EmptyView { container: view }
    }
}

impl EmptyView {
    #[inline]
    pub fn new() -> EmptyView {
        EmptyView::default()
    }
}

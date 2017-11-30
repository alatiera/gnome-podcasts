use gtk;

#[derive(Debug, Clone)]
pub struct EmptyView {
    container: gtk::Box,
}

impl EmptyView {
    pub fn new() -> EmptyView {
        let builder = gtk::Builder::new_from_resource("/org/gnome/hammond/gtk/empty_view.ui");
        let view: gtk::Box = builder.get_object("empty_view").unwrap();

        EmptyView { container: view }
    }
}

use gtk;

#[derive(Debug, Clone)]
pub struct Playback {
    pub container: gtk::Grid,
}

impl Default for Playback {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/playback.ui");
        let container = builder.get_object("wrapper").unwrap();

        Playback { container }
    }
}

impl Playback {
    pub fn new() -> Playback {
        Playback::default()
    }
}

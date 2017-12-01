use gtk::prelude::*;
use gtk::{Window, WindowType};

use headerbar::Header;
use content::Content;

#[derive(Debug)]
pub struct App<'a> {
    window: Window,
    header: Header,
    content: Content<'a>,
}

impl <'a>App<'a> {
    pub fn new() -> App<'a> {
        let window = Window::new(WindowType::Toplevel);
        let content = Content::new();
        let header = Header::new(content.stack.clone());

        window.set_default_size(1150, 650);
        window.connect_delete_event(|w, _| {
            w.destroy();
            Inhibit(false)
        });

        window.set_titlebar(&header.container);
        window.add(&content.stack);

        window.show_all();
        window.activate();

        App {
            window,
            header,
            content,
        }
    }
}

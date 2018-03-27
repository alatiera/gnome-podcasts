use gio;
use gio::SettingsExt;
use gtk;
use gtk::GtkWindowExt;

pub struct WindowGeometry {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    is_maximized: bool,
}

impl WindowGeometry {
    pub fn from_window(window: &gtk::Window) -> WindowGeometry {
        let position = window.get_position();
        let size = window.get_size();
        let left = position.0;
        let top = position.1;
        let width = size.0;
        let height = size.1;
        let is_maximized = window.is_maximized();

        WindowGeometry {
            left,
            top,
            width,
            height,
            is_maximized,
        }
    }

    pub fn from_settings(settings: &gio::Settings) -> WindowGeometry {
        let top = settings.get_int("persist-window-geometry-top");
        let left = settings.get_int("persist-window-geometry-left");
        let width = settings.get_int("persist-window-geometry-width");
        let height = settings.get_int("persist-window-geometry-height");
        let is_maximized = settings.get_boolean("persist-window-geometry-maximized");

        WindowGeometry {
            left,
            top,
            width,
            height,
            is_maximized,
        }
    }

    pub fn apply(&self, window: &gtk::Window) {
        if self.width > 0 && self.height > 0 {
            window.resize(self.width, self.height);
        }

        if self.is_maximized {
            window.maximize();
        } else if self.top > 0 && self.left > 0 {
            window.move_(self.left, self.top);
        }
    }

    pub fn write(&self, settings: &gio::Settings) {
        settings.set_int("persist-window-geometry-left", self.left);
        settings.set_int("persist-window-geometry-top", self.top);
        settings.set_int("persist-window-geometry-width", self.width);
        settings.set_int("persist-window-geometry-height", self.height);
        settings.set_boolean("persist-window-geometry-maximized", self.is_maximized);
    }
}

// #[test]
// fn test_apply_window_geometry() {
//     gtk::init().expect("Error initializing gtk.");

//     let window = gtk::Window::new(gtk::WindowType::Toplevel);
//     let _geometry = WindowGeometry {
//         left: 0,
//         top: 0,
//         width: 100,
//         height: 100,
//         is_maximized: true
//     };

//     assert!(!window.is_maximized());

//     window.show();
// window.activate();
//     geometry.apply(&window);

//     assert!(window.is_maximized());
// }

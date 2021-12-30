use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

#[derive(Debug, Default)]
pub struct ShortDescLayoutPriv {}

#[glib::object_subclass]
impl ObjectSubclass for ShortDescLayoutPriv {
    const NAME: &'static str = "PdShortDescLayout";
    type Type = ShortDescLayout;
    type ParentType = gtk::LayoutManager;
}

impl ObjectImpl for ShortDescLayoutPriv {}
impl LayoutManagerImpl for ShortDescLayoutPriv {
    fn allocate(
        &self,
        _layout_manager: &Self::Type,
        widget: &gtk::Widget,
        width: i32,
        height: i32,
        baseline: i32,
    ) {
        if let Some(label) = widget.first_child() {
            if label.is_visible() {
                label.allocate(width, height, baseline, None);

                let value = label
                    .downcast::<gtk::Label>()
                    .unwrap()
                    .layout()
                    .is_ellipsized()
                    .to_value();
                widget.emit_by_name("is-ellipsized", &[&value]).unwrap();
            }
        }
    }
    fn measure(
        &self,
        _layout_manager: &Self::Type,
        widget: &gtk::Widget,
        orientation: gtk::Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        if let Some(label) = widget.first_child() {
            label.measure(orientation, for_size)
        } else {
            (0, 0, -1, -1)
        }
    }
}

glib::wrapper! {
    pub struct ShortDescLayout(ObjectSubclass<ShortDescLayoutPriv>)
        @extends gtk::LayoutManager;
}

impl Default for ShortDescLayout {
    fn default() -> Self {
        glib::Object::new(&[]).unwrap()
    }
}

impl ShortDescLayout {}

use glib::subclass::Signal;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use once_cell::sync::Lazy;

use crate::widgets::ShortDescLayout;

#[derive(Debug, Default)]
pub struct ShortDescPriv {
    pub label: gtk::Label,
}

#[glib::object_subclass]
impl ObjectSubclass for ShortDescPriv {
    const NAME: &'static str = "PdShortDesc";
    type Type = ShortDesc;
    type ParentType = gtk::Widget;
}

impl ObjectImpl for ShortDescPriv {
    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder(
                "is-ellipsized",
                &[<bool>::static_type().into()],
                <()>::static_type().into(),
            )
            .flags(glib::SignalFlags::ACTION)
            .build()]
        });
        SIGNALS.as_ref()
    }

    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);
        self.label.set_parent(obj);

        self.label.set_valign(gtk::Align::Center);
        self.label.set_halign(gtk::Align::Center);
        self.label.set_use_markup(true);
        self.label.set_wrap(true);
        self.label.set_lines(4);
        self.label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        self.label.set_justify(gtk::Justification::Center);
        self.label.set_ellipsize(gtk::pango::EllipsizeMode::End);

        let layout = ShortDescLayout::default();
        obj.set_layout_manager(Some(&layout));
    }

    fn dispose(&self, _obj: &Self::Type) {
        self.label.unparent();
    }
}

impl WidgetImpl for ShortDescPriv {}

glib::wrapper! {
    pub struct ShortDesc(ObjectSubclass<ShortDescPriv>)
        @extends gtk::Widget;
}

impl Default for ShortDesc {
    fn default() -> Self {
        glib::Object::new(&[]).unwrap()
    }
}

impl ShortDesc {
    pub fn label(&self) -> glib::GString {
        let self_ = ShortDescPriv::from_instance(self);
        self_.label.label()
    }

    pub fn set_label(&self, label: &str) {
        let self_ = ShortDescPriv::from_instance(self);
        self_.label.set_markup(label);
    }
}

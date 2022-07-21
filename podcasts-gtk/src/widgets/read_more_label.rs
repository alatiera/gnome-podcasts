use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use once_cell::sync::Lazy;
use std::cell::Cell;

use crate::i18n::i18n;

#[derive(Debug, Default)]
pub struct ReadMoreLabelPriv {
    pub short_label: gtk::Label,
    pub short_desc: gtk::Box,
    pub long_label: gtk::Label,
    pub button: gtk::Button,

    pub expanded: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for ReadMoreLabelPriv {
    const NAME: &'static str = "PdReadMoreLabel";
    type Type = ReadMoreLabel;
    type ParentType = gtk::Widget;
}

impl ObjectImpl for ReadMoreLabelPriv {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        self.button.set_label(&i18n("Read More"));
        self.button.set_halign(gtk::Align::Center);
        self.button
            .connect_clicked(glib::clone!(@weak obj => move |button| {
                obj.set_expanded(true);
            }));

        self.short_label.set_halign(gtk::Align::Center);
        self.short_label.set_valign(gtk::Align::Center);
        self.short_label.set_use_markup(true);
        self.short_label.set_wrap(true);
        // See https://gitlab.gnome.org/GNOME/gtk/-/issues/4714.
        self.short_label.set_lines(4);
        self.short_label
            .set_wrap_mode(gtk::pango::WrapMode::WordChar);
        self.short_label.set_justify(gtk::Justification::Center);
        self.short_label
            .set_ellipsize(gtk::pango::EllipsizeMode::End);

        self.short_desc.set_orientation(gtk::Orientation::Vertical);
        self.short_desc.set_spacing(6);
        self.short_desc.append(&self.short_label);
        self.short_desc.append(&self.button);
        self.short_desc.set_child_visible(true);

        self.long_label.set_use_markup(true);
        self.long_label.set_justify(gtk::Justification::Center);
        self.long_label.set_wrap(true);
        self.long_label
            .set_wrap_mode(gtk::pango::WrapMode::WordChar);
        self.long_label.set_valign(gtk::Align::Center);
        self.long_label.set_halign(gtk::Align::Center);
        self.long_label.set_child_visible(false);

        self.long_label.set_parent(obj);
        self.short_desc.set_parent(obj);
    }

    fn dispose(&self, _obj: &Self::Type) {
        self.short_desc.unparent();
        self.long_label.unparent();
    }
}

impl WidgetImpl for ReadMoreLabelPriv {
    fn measure(
        &self,
        widget: &Self::Type,
        orientation: gtk::Orientation,
        for_size: i32,
    ) -> (i32, i32, i32, i32) {
        if self.expanded.get() {
            let (min_h, nat_h, min_b, nat_b) = self.long_label.measure(orientation, for_size);
            if orientation == gtk::Orientation::Vertical {
                (nat_h, nat_h, min_b, nat_b)
            } else {
                (min_h, nat_h, min_b, nat_b)
            }
        } else {
            self.short_desc.measure(orientation, for_size)
        }
    }

    fn request_mode(&self, widget: &Self::Type) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::WidthForHeight
    }

    fn size_allocate(&self, widget: &Self::Type, width: i32, height: i32, baseline: i32) {
        let long_nat_h = self.long_label.measure(gtk::Orientation::Vertical, width).1;

        // If we have enough space to allocate the long label, we directly
        // allocate it.
        if long_nat_h < height {
            widget.set_expanded_inner(true);
        }

        if self.expanded.get() {
            self.long_label.allocate(width, height, baseline, None);
        } else {
            self.short_desc.allocate(width, height, baseline, None);
        }
    }
}

glib::wrapper! {
    pub struct ReadMoreLabel(ObjectSubclass<ReadMoreLabelPriv>)
        @extends gtk::Widget;
}

impl Default for ReadMoreLabel {
    fn default() -> Self {
        glib::Object::new(&[]).unwrap()
    }
}

impl ReadMoreLabel {
    fn set_expanded_inner(&self, expanded: bool) {
        let imp = self.imp();
        if expanded == imp.expanded.replace(expanded) {
            return;
        }

        // This should be only set once.
        if !expanded {
            return;
        }

        imp.long_label.set_child_visible(expanded);
        imp.short_desc.set_child_visible(!expanded);
    }

    fn set_expanded(&self, expanded: bool) {
        self.set_expanded_inner(expanded);

        self.queue_resize();
    }

    fn expanded(&self) -> bool {
        self.imp().expanded.get()
    }

    pub fn set_label(&self, label: &str) {
        let imp = self.imp();
        let markup = glib::markup_escape_text(label);

        let lines: Vec<&str> = markup.lines().collect();
        if !lines.is_empty() {
            imp.short_label.set_markup(lines[0]);
        }

        imp.long_label.set_markup(&markup);
    }
}

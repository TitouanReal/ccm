use std::cell::RefCell;

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, Object};

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Event)]
    pub struct Event {
        #[property(get, set)]
        name: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Event {
        const NAME: &'static str = "Event";
        type Type = super::Event;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Event {}
}

glib::wrapper! {
    pub struct Event(ObjectSubclass<imp::Event>);
}

impl Event {
    pub(crate) fn new(name: &str) -> Self {
        glib::Object::builder().property("name", name).build()
    }
}

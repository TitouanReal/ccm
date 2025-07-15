use std::cell::{OnceCell, RefCell};

use gdk::{
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
};

use crate::Manager;

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Event)]
    pub struct Event {
        #[property(get, construct_only)]
        manager: OnceCell<Manager>,
        #[property(get, construct_only)]
        uri: OnceCell<String>,
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        description: RefCell<String>,
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
    pub(crate) fn new(manager: &Manager, uri: &str, name: &str, description: &str) -> Self {
        glib::Object::builder()
            .property("manager", manager)
            .property("uri", uri)
            .property("name", name)
            .property("description", description)
            .build()
    }
}

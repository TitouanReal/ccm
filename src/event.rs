use std::{
    cell::{OnceCell, RefCell},
    sync::LazyLock,
};

use gdk::{
    glib::{self, Object, closure_local, subclass::Signal},
    prelude::*,
    subclass::prelude::*,
};

use crate::{Calendar, Manager, Timeframe};

mod imp {

    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Event)]
    pub struct Event {
        #[property(get, construct_only)]
        manager: OnceCell<Manager>,
        #[property(get, construct_only)]
        calendar: OnceCell<Calendar>,
        #[property(get, construct_only)]
        uri: OnceCell<String>,
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get, set)]
        description: RefCell<String>,
        #[property(get, set)]
        timeframe: RefCell<Option<Timeframe>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Event {
        const NAME: &'static str = "Event";
        type Type = super::Event;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Event {
        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> =
                LazyLock::new(|| vec![Signal::builder("deleted").build()]);
            SIGNALS.as_ref()
        }
    }
}

glib::wrapper! {
    pub struct Event(ObjectSubclass<imp::Event>);
}

impl Event {
    /// Create a new event from its properties.
    pub(crate) fn new(
        manager: &Manager,
        calendar: &Calendar,
        uri: &str,
        name: &str,
        description: &str,
        timeframe: &Timeframe,
    ) -> Self {
        glib::Object::builder()
            .property("manager", manager)
            .property("calendar", calendar)
            .property("uri", uri)
            .property("name", name)
            .property("description", description)
            .property("timeframe", timeframe)
            .build()
    }

    /// Signal that this event was deleted.
    pub(super) fn emit_deleted(&self) {
        self.emit_by_name::<()>("deleted", &[]);
    }

    /// Connect to the signal emitted when this event is deleted.
    pub fn connect_deleted<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "deleted",
            true,
            closure_local!(|obj: Self| {
                f(&obj);
            }),
        )
    }
}

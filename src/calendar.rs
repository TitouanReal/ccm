use std::{
    cell::{OnceCell, RefCell},
    sync::LazyLock,
};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk::{self, RGBA},
    gio::ListStore,
    glib::{self, Object, closure_local, subclass::Signal},
};

use crate::{Event, Manager};

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Calendar)]
    pub struct Calendar {
        #[property(get, set, construct_only)]
        manager: OnceCell<Manager>,
        #[property(get, construct_only)]
        uri: RefCell<String>,
        #[property(get, set)]
        name: RefCell<String>,
        // TODO: Remove the Option
        #[property(get, set)]
        color: RefCell<Option<RGBA>>,
        #[property(get)]
        events: OnceCell<ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Calendar {
        const NAME: &'static str = "Calendar";
        type Type = super::Calendar;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Calendar {
        fn constructed(&self) {
            self.parent_constructed();

            self.events.get_or_init(ListStore::new::<Event>);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> =
                LazyLock::new(|| vec![Signal::builder("deleted").build()]);
            SIGNALS.as_ref()
        }
    }

    impl Calendar {
        pub fn events(&self) -> &ListStore {
            self.events.get().expect("events should be initialized")
        }
    }
}

glib::wrapper! {
    pub struct Calendar(ObjectSubclass<imp::Calendar>);
}

impl Calendar {
    pub(crate) fn new(manager: &Manager, uri: &str, name: &str, color: gdk::RGBA) -> Self {
        glib::Object::builder()
            .property("manager", manager)
            .property("uri", uri)
            .property("name", name)
            .property("color", Some(color))
            .build()
    }

    pub fn update(&self, name: Option<&str>, color: Option<gdk::RGBA>) {
        // TODO: dispatch to relevant provider instead
        self.manager().update_calendar(&self.uri(), name, color);
    }

    pub(crate) fn emit_updated(&self, name: &str, color: gdk::RGBA) {
        // TODO: Manual notification
        self.set_property("name", name);
        self.set_property("color", Some(color));
    }

    /// Deletes the calendar from the database.
    pub fn delete(&self) {
        // TODO: dispatch to relevant provider instead
        self.manager().delete_calendar(&self.uri());
    }

    /// Signal that this calendar was deleted.
    pub(super) fn emit_deleted(&self) {
        self.emit_by_name::<()>("deleted", &[]);
    }

    /// Connect to the signal emitted when this calendar is deleted.
    pub fn connect_deleted<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_closure(
            "deleted",
            true,
            closure_local!(|obj: Self| {
                f(&obj);
            }),
        )
    }

    pub(crate) fn add_event(&self, event: &Event) {
        self.imp().events().append(event);
    }
}

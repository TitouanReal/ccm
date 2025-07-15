use std::{
    cell::{OnceCell, RefCell},
    sync::LazyLock,
};

use gdk::{
    RGBA,
    gio::ListStore,
    glib::{self, Object, clone, closure_local, subclass::Signal},
    prelude::*,
    subclass::prelude::*,
};
use tracing::info;

use crate::{Collection, Event, Manager};

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Calendar)]
    pub struct Calendar {
        #[property(get, construct_only)]
        manager: OnceCell<Manager>,
        #[property(get, construct_only)]
        collection: OnceCell<Collection>,
        #[property(get, construct_only)]
        uri: OnceCell<String>,
        #[property(get, set, explicit_notify)]
        name: RefCell<String>,
        // TODO: Remove the Option
        #[property(get, set, explicit_notify)]
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
    /// Create a calendar from its properties.
    pub(crate) fn new(
        manager: &Manager,
        collection: &Collection,
        uri: &str,
        name: &str,
        color: gdk::RGBA,
    ) -> Self {
        glib::Object::builder()
            .property("manager", manager)
            .property("collection", collection)
            .property("uri", uri)
            .property("name", name)
            .property("color", Some(color))
            .build()
    }

    /// Ask the backend to update this calendar. Properties with a None value will be left
    /// unchanged.
    pub fn update(&self, name: Option<&str>, color: Option<gdk::RGBA>) {
        // TODO: dispatch to relevant provider instead
        self.manager().update_calendar(&self.uri(), name, color);
    }

    /// TODO
    pub(crate) fn emit_updated(&self, name: &str, color: gdk::RGBA) {
        let uri = self.uri();
        if name != self.name() {
            self.set_property("name", name);
            info!("Calendar {uri} updated to name {name}");
            self.notify_name();
        }
        if color != self.color().unwrap() {
            self.set_property("color", Some(color));
            info!("Calendar {uri} updated to color {color}");
            self.notify_color();
        }
    }

    /// Ask the backend to delete this calendar.
    pub fn delete(&self) {
        // TODO: dispatch to relevant provider instead
        self.manager().delete_calendar(&self.uri());
    }

    /// Signal that this calendar was deleted.
    pub(super) fn emit_deleted(&self) {
        for event in self.events().iter::<Event>() {
            event
                .expect("Model should not be mutated during iteration")
                .emit_deleted();
        }

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

    /// Add an event to this calendar.
    pub(crate) fn add_event(&self, event: &Event) {
        self.imp().events().append(event);

        event.connect_deleted(clone!(
            #[weak(rename_to = obj)]
            self,
            move |event| {
                let index = obj.events().find(event).expect("Event should be found");
                obj.events().remove(index);
            }
        ));
    }

    /// Ask the backend to create a new event in this calendar.
    pub fn create_event(&self, name: &str, description: &str) {
        // TODO: dispatch to relevant provider instead
        self.manager().create_event(&self.uri(), name, description);
    }
}

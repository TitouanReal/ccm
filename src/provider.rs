use std::cell::{OnceCell, RefCell};

use gdk::{
    gio::ListStore,
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
};

use crate::{Collection, Manager};

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Provider)]
    pub struct Provider {
        #[property(get, construct_only)]
        manager: OnceCell<Manager>,
        #[property(get, construct_only)]
        uri: OnceCell<String>,
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get)]
        collections: OnceCell<ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Provider {
        const NAME: &'static str = "Provider";
        type Type = super::Provider;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Provider {
        fn constructed(&self) {
            self.parent_constructed();

            self.collections.get_or_init(ListStore::new::<Collection>);
        }
    }

    impl Provider {
        pub fn collections(&self) -> &ListStore {
            self.collections
                .get()
                .expect("collections should be initialized")
        }
    }
}

glib::wrapper! {
    pub struct Provider(ObjectSubclass<imp::Provider>);
}

impl Provider {
    /// Create a provider from its properties.
    pub(crate) fn new(manager: &Manager, uri: &str, name: &str) -> Self {
        glib::Object::builder()
            .property("manager", manager)
            .property("uri", uri)
            .property("name", name)
            .build()
    }

    /// Add a collection to this provider.
    pub(crate) fn add_collection(&self, collection: &Collection) {
        self.imp().collections().append(collection);
    }
}

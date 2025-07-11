use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gio::ListStore,
    glib::{self, Object},
};

use crate::Collection;

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Provider)]
    pub struct Provider {
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
    /// Create a provider resource from its properties.
    pub(crate) fn new(name: &str) -> Self {
        glib::Object::builder().property("name", name).build()
    }

    pub(crate) fn add_collection(&self, collection: &Collection) {
        self.imp().collections().append(collection);
    }
}

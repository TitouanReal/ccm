use std::cell::{OnceCell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gdk::RGBA,
    gio::{self, ListStore},
    glib::{self, Object, clone},
};

use crate::{Calendar, Manager};

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Collection)]
    pub struct Collection {
        #[property(get, set, construct_only)]
        manager: OnceCell<Manager>,
        #[property(get, construct_only)]
        uri: RefCell<String>,
        #[property(get, set)]
        name: RefCell<String>,
        #[property(get)]
        calendars: OnceCell<ListStore>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Collection {
        const NAME: &'static str = "Collection";
        type Type = super::Collection;
        type ParentType = Object;
        type Interfaces = (gio::ListModel,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for Collection {
        fn constructed(&self) {
            self.parent_constructed();

            self.calendars.get_or_init(ListStore::new::<Calendar>);
        }
    }

    impl ListModelImpl for Collection {
        fn item_type(&self) -> glib::Type {
            Calendar::static_type()
        }
        fn n_items(&self) -> u32 {
            self.calendars().n_items()
        }
        fn item(&self, position: u32) -> Option<glib::Object> {
            self.calendars().item(position)
        }
    }

    impl Collection {
        pub fn calendars(&self) -> &ListStore {
            self.calendars
                .get()
                .expect("calendars should be initialized")
        }
    }
}

glib::wrapper! {
    pub struct Collection(ObjectSubclass<imp::Collection>)
        @implements gio::ListModel;
}

impl Collection {
    pub(crate) fn new(manager: &Manager, uri: &str, name: &str) -> Self {
        glib::Object::builder()
            .property("manager", manager)
            .property("uri", uri)
            .property("name", name)
            .build()
    }

    pub(crate) fn add_calendar(&self, calendar: &Calendar) {
        self.imp().calendars().append(calendar);

        calendar.connect_deleted(clone!(
            #[weak(rename_to = obj)]
            self,
            move |calendar| {
                let index = obj
                    .calendars()
                    .find(calendar)
                    .expect("Calendar should be found");
                obj.calendars().remove(index);
            }
        ));
    }

    pub fn create_calendar(&self, name: &str, color: RGBA) {
        // TODO: dispatch to relevant provider instead
        self.manager().create_calendar(&self.uri(), name, color);
    }
}

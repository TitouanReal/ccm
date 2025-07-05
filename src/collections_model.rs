use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use std::cell::RefCell;

use super::collection::Collection;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct CollectionsModel(pub RefCell<Vec<Collection>>);

    #[glib::object_subclass]
    impl ObjectSubclass for CollectionsModel {
        const NAME: &'static str = "CollectionsModel";
        type Type = super::CollectionsModel;
        type Interfaces = (gio::ListModel,);
    }

    impl ObjectImpl for CollectionsModel {}
    impl ListModelImpl for CollectionsModel {
        fn item_type(&self) -> glib::Type {
            Collection::static_type()
        }
        fn n_items(&self) -> u32 {
            self.0.borrow().len() as u32
        }
        fn item(&self, position: u32) -> Option<glib::Object> {
            self.0
                .borrow()
                .get(position as usize)
                .map(|o| o.clone().upcast::<glib::Object>())
        }
    }
}

glib::wrapper! {
    pub struct CollectionsModel(ObjectSubclass<imp::CollectionsModel>)
        @implements gio::ListModel;
}

impl CollectionsModel {
    pub fn append(&self, collection: &Collection) {
        let pos = {
            let mut data = self.imp().0.borrow_mut();
            data.push(collection.clone());
            (data.len() - 1) as u32
        };
        self.items_changed(pos, 0, 1);
    }

    pub fn splice(&self, collections: &[Collection]) {
        let len = collections.len();
        let pos = {
            let mut data = self.imp().0.borrow_mut();
            let pos = data.len();
            data.extend_from_slice(collections);
            pos as u32
        };
        self.items_changed(pos, 0, len as u32);
    }

    pub fn remove(&self, pos: u32) {
        self.imp().0.borrow_mut().remove(pos as usize);
        self.items_changed(pos, 1, 0);
    }
}

impl Default for CollectionsModel {
    fn default() -> Self {
        glib::Object::new()
    }
}

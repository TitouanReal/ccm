use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use gdk::{
    RGBA,
    gio::{self, BusType, DBusCallFlags, DBusProxy, DBusProxyFlags, ListStore},
    glib::{self, Object, clone},
    prelude::*,
    subclass::prelude::*,
};
use tracing::{debug, info, warn};
use tsparql::{Notifier, NotifierEvent, NotifierEventType, SparqlConnection, prelude::*};

use crate::{
    Calendar, Collection, Event, Provider, Resource, collections_model::CollectionsModel,
    pre_resource::PreResource, spawn,
};

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Manager)]
    pub struct Manager {
        read_connection: OnceCell<SparqlConnection>,
        write_connection: OnceCell<DBusProxy>,
        notifier: OnceCell<Notifier>,
        resource_pool: OnceCell<Mutex<HashMap<String, Resource>>>,
        #[property(get)]
        collections_model: OnceCell<CollectionsModel>,
        events_handler: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Manager {
        const NAME: &'static str = "Manager";
        type Type = super::Manager;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Manager {
        fn constructed(&self) {
            self.parent_constructed();

            self.read_connection.get_or_init(|| {
                SparqlConnection::bus_new("io.gitlab.TitouanReal.CcmRead", None, None).unwrap()
            });

            self.write_connection.get_or_init(|| {
                DBusProxy::for_bus_sync(
                    BusType::Session,
                    DBusProxyFlags::NONE,
                    None,
                    "io.gitlab.TitouanReal.CcmWrite",
                    "/io/gitlab/TitouanReal/CcmWrite/Provider",
                    "io.gitlab.TitouanReal.CcmWrite.Provider",
                    None::<&gio::Cancellable>,
                )
                .unwrap()
            });

            self.notifier
                .get_or_init(|| SparqlConnection::create_notifier(self.read_connection()).unwrap());

            self.resource_pool
                .get_or_init(|| Mutex::new(HashMap::new()));

            self.collections_model
                .get_or_init(CollectionsModel::default);

            spawn!(clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    imp.retrieve_resources();
                }
            ));

            self.events_handler
                .replace(Some(self.notifier().connect_events(clone!(
                    #[weak(rename_to = imp)]
                    self,
                    move |_notifier: &tsparql::Notifier,
                          _service: Option<&str>,
                          _graph: Option<&str>,
                          events: Vec<NotifierEvent>| {
                        imp.handle_notifier_events(events);
                    },
                ))));
        }
    }

    impl Manager {
        pub(super) fn read_connection(&self) -> &SparqlConnection {
            self.read_connection
                .get()
                .expect("read connection should be initialized")
        }

        pub(super) fn write_connection(&self) -> &DBusProxy {
            self.write_connection
                .get()
                .expect("write connection should be initialized")
        }

        fn notifier(&self) -> &Notifier {
            self.notifier.get().expect("notifier should be initialized")
        }

        // TODO: Do not lock the mutex here
        pub(super) fn resource_pool(&self) -> MutexGuard<'_, HashMap<String, Resource>> {
            self.resource_pool
                .get()
                .expect("resource pool should be initialized")
                .lock()
                .unwrap()
        }

        fn retrieve_resources(&self) {
            self.retrieve_providers();
            self.retrieve_collections();
            self.retrieve_calendars();
            self.retrieve_events();
        }

        fn retrieve_providers(&self) {
            let cursor = self
                .read_connection()
                .query(
                    "SELECT ?uri ?name
                    WHERE {
                        ?uri a ccm:Provider ;
                            ccm:providerName ?name .
                    }",
                    None::<&gio::Cancellable>,
                )
                .expect("Failed to retrieve providers");

            while let Ok(true) = cursor.next(None::<&gio::Cancellable>) {
                let uri = cursor.string(0).expect("Query should return a URI");
                let name = cursor.string(1).expect("Query should return a name");
                let provider = Provider::new(&self.obj(), &uri, &name);

                self.resource_pool()
                    .insert(uri.to_string(), Resource::Provider(provider));

                info!("Found provider: uri: \"{uri}\", name: \"{name}\"");
            }
        }

        fn retrieve_collections(&self) {
            let cursor = self
                .read_connection()
                .query(
                    "SELECT ?uri ?provider_uri ?name
                    WHERE {
                        ?uri a ccm:Collection ;
                            ccm:provider ?provider_uri ;
                            ccm:collectionName ?name .
                    }",
                    None::<&gio::Cancellable>,
                )
                .expect("Failed to retrieve collections");

            while let Ok(true) = cursor.next(None::<&gio::Cancellable>) {
                let uri = cursor.string(0).expect("Query should return a URI");
                let provider_uri = cursor
                    .string(1)
                    .expect("Query should return a provider URI");
                let name = cursor.string(2).expect("Query should return a name");

                let Some(Resource::Provider(provider)) =
                    self.resource_pool().get(provider_uri.as_str()).cloned()
                else {
                    warn!("Collection \"{uri}\" has an invalid provider \"{provider_uri}\"");
                    continue;
                };

                let collection = Collection::new(&self.obj(), &provider, &uri, &name);

                provider.add_collection(&collection);
                self.obj().collections_model().append(&collection);
                self.resource_pool()
                    .insert(uri.to_string(), Resource::Collection(collection));

                info!("Found collection: uri: \"{uri}\", name: \"{name}\"");
            }
        }

        fn retrieve_calendars(&self) {
            let cursor = self
                .read_connection()
                .query(
                    "SELECT ?uri ?collection_uri ?name ?color
                    WHERE {
                        ?uri a ccm:Calendar ;
                            ccm:collection ?collection_uri ;
                            ccm:calendarName ?name ;
                            ccm:color ?color .
                    }",
                    None::<&gio::Cancellable>,
                )
                .expect("Failed to retrieve calendars");

            while let Ok(true) = cursor.next(None::<&gio::Cancellable>) {
                let uri = cursor.string(0).expect("Query should return a URI");
                let collection_uri = cursor
                    .string(1)
                    .expect("Query should return a collection URI");
                let name = cursor.string(2).expect("Query should return a name");
                let color = cursor.string(3).expect("Query should return a color");

                let Some(Resource::Collection(collection)) =
                    self.resource_pool().get(collection_uri.as_str()).cloned()
                else {
                    warn!("Calendar \"{uri}\" has an invalid collection \"{collection_uri}\"");
                    continue;
                };

                let calendar = Calendar::new(
                    &self.obj(),
                    &collection,
                    &uri,
                    &name,
                    color.parse().expect("Color should be a valid color string"),
                );

                collection.add_calendar(&calendar);
                self.resource_pool()
                    .insert(uri.to_string(), Resource::Calendar(calendar));

                info!("Found calendar: uri: \"{uri}\", name: \"{name}\", color: \"{color}\"");
            }
        }

        fn retrieve_events(&self) {
            let cursor = self
                .read_connection()
                .query(
                    "SELECT ?uri ?calendar_uri ?name ?description
                    WHERE {
                        ?uri a ccm:Event ;
                            ccm:calendar ?calendar_uri ;
                            ccm:eventName ?name ;
                            ccm:eventDescription ?description .
                    }",
                    None::<&gio::Cancellable>,
                )
                .expect("Failed to retrieve events");

            while let Ok(true) = cursor.next(None::<&gio::Cancellable>) {
                let uri = cursor.string(0).expect("Query should return a URI");
                let calendar_uri = cursor
                    .string(1)
                    .expect("Query should return a calendar URI");
                let name = cursor.string(2).expect("Query should return a name");
                let description = cursor.string(3).expect("Query should return a description");

                let Some(Resource::Calendar(calendar)) =
                    self.resource_pool().get(calendar_uri.as_str()).cloned()
                else {
                    warn!("Event \"{uri}\" has an invalid calendar \"{calendar_uri}\"");
                    continue;
                };

                let event = Event::new(&self.obj(), &calendar, &uri, &name, &description);

                calendar.add_event(&event);
                self.resource_pool()
                    .insert(uri.to_string(), Resource::Event(event));

                info!(
                    "Found event: uri: \"{uri}\", name: \"{name}\", description: \"{description}\""
                );
            }
        }

        fn handle_notifier_events(&self, events: Vec<NotifierEvent>) {
            let num_events = events.len();
            if num_events == 1 {
                debug!("Starting to handle 1 event");
            } else {
                debug!("Starting to handle {num_events} events");
            }

            let mut resource_pool = self.resource_pool();

            let mut created_uris = Vec::new();
            let mut updated_uris = Vec::new();
            let mut deleted_uris = Vec::new();

            for mut event in events {
                match event.event_type() {
                    NotifierEventType::Create => {
                        created_uris.push(event.urn().unwrap());
                    }
                    NotifierEventType::Update => {
                        updated_uris.push(event.urn().unwrap());
                    }
                    NotifierEventType::Delete => {
                        deleted_uris.push(event.urn().unwrap());
                    }
                    _ => {
                        warn!("Unknown event type: {:?}", event.event_type());
                    }
                }
            }

            match created_uris.len() {
                0 => {}
                1 => {
                    debug!("Handling 1 \"Create\" events");
                }
                num_create_events => {
                    debug!("Handling {num_create_events} \"Create\" events");
                }
            }
            let created_resources = created_uris
                .into_iter()
                .map(|uri| PreResource::from_uri(self.read_connection(), &uri).unwrap())
                .collect::<Vec<_>>();

            // Create providers
            for pre_provider in created_resources.iter().filter_map(|pre_resource| {
                if let PreResource::Provider(pre_provider) = pre_resource {
                    Some(pre_provider)
                } else {
                    None
                }
            }) {
                let provider = Provider::new(&self.obj(), &pre_provider.uri, &pre_provider.name);
                let provider_uri = pre_provider.uri.clone();
                resource_pool.insert(provider_uri, Resource::Provider(provider));

                info!(
                    "Provider created: uri: \"{}\", name: \"{}\"",
                    pre_provider.uri, pre_provider.name
                );
            }

            // Create collections
            for pre_collection in created_resources.iter().filter_map(|pre_resource| {
                if let PreResource::Collection(pre_collection) = pre_resource {
                    Some(pre_collection)
                } else {
                    None
                }
            }) {
                let collection_uri = pre_collection.uri.clone();
                let provider_uri = pre_collection.provider_uri.clone();

                if let Some(Resource::Provider(provider)) = resource_pool.get(&provider_uri) {
                    let collection = Collection::new(
                        &self.obj(),
                        provider,
                        &pre_collection.uri,
                        &pre_collection.name,
                    );
                    provider.add_collection(&collection);
                    resource_pool.insert(collection_uri, Resource::Collection(collection));

                    info!(
                        "Collection created: uri: \"{}\", name: \"{}\"",
                        pre_collection.uri, pre_collection.name
                    );
                } else {
                    warn!(
                        "Collection {collection_uri} has provider {provider_uri} but it does not exist"
                    );
                }
            }

            // Create calendars
            for pre_calendar in created_resources.iter().filter_map(|pre_resource| {
                if let PreResource::Calendar(pre_calendar) = pre_resource {
                    Some(pre_calendar)
                } else {
                    None
                }
            }) {
                let calendar_uri = pre_calendar.uri.clone();
                let collection_uri = pre_calendar.collection_uri.clone();

                if let Some(Resource::Collection(collection)) = resource_pool.get(&collection_uri) {
                    let calendar = Calendar::new(
                        &self.obj(),
                        collection,
                        &pre_calendar.uri,
                        &pre_calendar.name,
                        pre_calendar.color,
                    );
                    collection.add_calendar(&calendar);
                    resource_pool.insert(calendar_uri, Resource::Calendar(calendar));

                    info!(
                        "Calendar created: uri: \"{}\", name: \"{}\"",
                        pre_calendar.uri, pre_calendar.name
                    );
                } else {
                    warn!(
                        "Calendar {calendar_uri} has collection {collection_uri} but it does not exist"
                    );
                }
            }

            // Create events
            for pre_event in created_resources.iter().filter_map(|pre_resource| {
                if let PreResource::Event(pre_event) = pre_resource {
                    Some(pre_event)
                } else {
                    None
                }
            }) {
                let event_uri = pre_event.uri.to_string();
                let calendar_uri = pre_event.calendar_uri.clone();

                if let Some(Resource::Calendar(calendar)) = resource_pool.get(&calendar_uri) {
                    let event = Event::new(
                        &self.obj(),
                        calendar,
                        &pre_event.uri,
                        &pre_event.name,
                        &pre_event.description,
                    );
                    calendar.add_event(&event);
                    resource_pool.insert(event_uri, Resource::Event(event));

                    info!(
                        "Event created: uri: \"{}\", name: \"{}\"",
                        pre_event.uri, pre_event.name
                    );
                } else {
                    warn!("Event {event_uri} has calendar {calendar_uri} but it does not exist");
                }
            }

            match updated_uris.len() {
                0 => {}
                1 => {
                    debug!("Handling 1 \"Update\" events");
                }
                num_update_events => {
                    debug!("Handling {num_update_events} \"Update\" events");
                }
            }
            let update_events = updated_uris
                .into_iter()
                .map(|uri| {
                    let old = resource_pool.get(uri.as_str()).unwrap().to_owned();
                    let new = PreResource::from_uri(self.read_connection(), &uri).unwrap();
                    (old, new)
                })
                .collect::<Vec<_>>();
            for update_event in update_events {
                match update_event {
                    (Resource::Provider(_old_provider), PreResource::Provider(_new_provider)) => {
                        todo!()
                    }
                    (
                        Resource::Collection(_old_collection),
                        PreResource::Collection(_new_collection),
                    ) => {
                        todo!()
                    }
                    (Resource::Calendar(old_calendar), PreResource::Calendar(new_calendar)) => {
                        old_calendar.emit_updated(&new_calendar.name, new_calendar.color);
                    }
                    (Resource::Event(_old_event), PreResource::Event(_new_event)) => {}
                    _ => {
                        todo!()
                    }
                }
            }

            match deleted_uris.len() {
                0 => {}
                1 => {
                    debug!("Handling 1 \"Delete\" events");
                }
                num_delete_events => {
                    debug!("Handling {num_delete_events} \"Delete\" events");
                }
            }
            for deleted_uri in deleted_uris {
                let Some(resource) = resource_pool.get(deleted_uri.as_str()).cloned() else {
                    warn!("Resource {deleted_uri} was deleted but is not found locally");
                    continue;
                };
                match resource {
                    Resource::Provider(_provider) => todo!(),
                    Resource::Collection(_collection) => todo!(),
                    Resource::Calendar(calendar) => {
                        // TODO: Emit deleted for events too
                        calendar.emit_deleted();
                    }
                    Resource::Event(_event) => todo!(),
                }
            }

            if num_events == 1 {
                debug!("Finished to handle 1 event");
            } else {
                debug!("Finished to handle {num_events} events");
            }
        }
    }
}

glib::wrapper! {
    pub struct Manager(ObjectSubclass<imp::Manager>);
}

impl Manager {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn find_resource(&self, uri: &str) -> Option<Resource> {
        self.imp().resource_pool().get(uri).cloned()
    }

    pub(crate) fn create_calendar(&self, collection_uri: &str, name: &str, color: RGBA) {
        // TODO: dispatch to relevant provider instead
        self.imp()
            .write_connection()
            .call_sync(
                "CreateCalendar",
                Some(&(collection_uri, name, &color.to_string()).to_variant()),
                DBusCallFlags::NONE,
                -1,
                None::<&gio::Cancellable>,
            )
            .unwrap();
    }

    pub(crate) fn update_calendar(&self, uri: &str, name: Option<&str>, color: Option<RGBA>) {
        // TODO: dispatch to relevant provider instead
        if let Some(name) = name {
            self.imp()
                .write_connection()
                .call_sync(
                    "UpdateCalendarName",
                    Some(&(uri, name).to_variant()),
                    DBusCallFlags::NONE,
                    -1,
                    None::<&gio::Cancellable>,
                )
                .unwrap();
        }
        if let Some(color) = color {
            self.imp()
                .write_connection()
                .call_sync(
                    "UpdateCalendarColor",
                    Some(&(uri, color.to_string()).to_variant()),
                    DBusCallFlags::NONE,
                    -1,
                    None::<&gio::Cancellable>,
                )
                .unwrap();
        }
    }

    pub(crate) fn delete_calendar(&self, uri: &str) {
        // TODO: dispatch to relevant provider instead
        self.imp()
            .write_connection()
            .call_sync(
                "DeleteCalendar",
                Some(&(uri,).to_variant()),
                DBusCallFlags::NONE,
                -1,
                None::<&gio::Cancellable>,
            )
            .unwrap();
    }

    pub(crate) fn create_event(&self, calendar_uri: &str, name: &str, description: &str) {
        // TODO: dispatch to relevant provider instead
        self.imp()
            .write_connection()
            .call_sync(
                "CreateEvent",
                Some(&(calendar_uri, name, description).to_variant()),
                DBusCallFlags::NONE,
                -1,
                None::<&gio::Cancellable>,
            )
            .unwrap();
    }

    pub fn search_events(&self, query: &str) -> ListStore {
        if query.is_empty() {
            return ListStore::new::<Event>();
        }

        let statement = self
            .imp()
            .read_connection()
            .query_statement(
                "SELECT ?uri
                WHERE {
                    ?uri a ccm:Event ;
                        fts:match ~query .
                }",
                None::<&gio::Cancellable>,
            )
            .expect("SPARQL should be valid")
            .expect("SPARQL should be valid");
        statement.bind_string("query", query);

        let cursor = match statement.execute(None::<&gio::Cancellable>) {
            Ok(cursor) => cursor,
            Err(err) => {
                warn!("Failed to search events: {err:?}");
                return ListStore::new::<Event>();
            }
        };

        let search_results = ListStore::new::<Event>();

        while let Ok(true) = cursor.next(None::<&gio::Cancellable>) {
            let uri = cursor.string(0).expect("Query should return a URI");

            let Some(Resource::Event(event)) =
                self.imp().resource_pool().get(uri.as_str()).cloned()
            else {
                warn!("Event \"{uri}\" is not in resource pool");
                continue;
            };

            search_results.append(&event);
        }

        search_results
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

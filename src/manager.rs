use std::{
    cell::{OnceCell, RefCell},
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use adw::subclass::prelude::*;
use gtk::{
    gdk::RGBA,
    gio::{self, BusType, DBusCallFlags, DBusProxy, DBusProxyFlags, ListStore, prelude::*},
    glib::{self, Object, clone},
};
use tracing::{debug, info, warn};
use tsparql::{Notifier, NotifierEvent, NotifierEventType, SparqlConnection, prelude::*};

use crate::{Calendar, Collection, Event, Provider, Resource, pre_resource::PreResource, spawn};

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Manager)]
    pub struct Manager {
        read_connection: OnceCell<SparqlConnection>,
        write_connection: OnceCell<DBusProxy>,
        notifier: OnceCell<tsparql::Notifier>,
        #[property(get)]
        collections: OnceCell<ListStore>,
        resource_pool: OnceCell<Mutex<HashMap<String, Resource>>>,
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

            self.collections.get_or_init(ListStore::new::<Collection>);

            spawn!(clone!(
                #[weak(rename_to = imp)]
                self,
                async move {
                    imp.refresh_resources();
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

    #[gtk::template_callbacks]
    impl Manager {
        fn read_connection(&self) -> &SparqlConnection {
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

        pub(super) fn resource_pool(&self) -> MutexGuard<'_, HashMap<String, Resource>> {
            self.resource_pool
                .get()
                .expect("resource pool should be initialized")
                .lock()
                .unwrap()
        }

        fn refresh_resources(&self) {
            let collection_cursor = match self.read_connection().query(
                "SELECT ?collection ?collection_name
                    FROM ccm:Calendar
                    WHERE {
                        ?collection a ccm:Collection;
                            rdfs:label ?collection_name.
                    }",
                None::<&gio::Cancellable>,
            ) {
                Ok(cursor) => cursor,
                Err(e) => {
                    warn!("Failed to execute query: {e}");
                    return;
                }
            };

            while let Ok(true) = collection_cursor.next(None::<&gio::Cancellable>) {
                let collection_uri = collection_cursor.string(0).unwrap();
                let collection_name = collection_cursor.string(1).unwrap();
                let collection = Collection::new(&self.obj(), &collection_uri, &collection_name);

                let maybe_old_resource = self.resource_pool().insert(
                    collection_uri.to_string(),
                    Resource::Collection(collection.clone()),
                );
                // URIs are unique. This should not happen.
                if maybe_old_resource.is_some() {
                    warn!("Encountered a duplicate URI \"{collection_uri}\"");
                }
                self.obj().collections().insert(0, &collection);

                info!("Found collection: uri: \"{collection_uri}\", name: \"{collection_name}\"");

                let statement = self
                    .read_connection()
                    .query_statement(
                        "SELECT ?calendar ?calendar_color ?calendar_name
                        FROM ccm:Calendar
                        WHERE {
                            ?calendar a ccm:Calendar ;
                                rdfs:label ?calendar_name ;
                                ccm:color ?calendar_color ;
                                ccm:collection ~collection_uri.
                        }",
                        None::<&gio::Cancellable>,
                    )
                    .unwrap()
                    .unwrap();
                statement.bind_string("collection_uri", &collection_uri);

                let calendar_cursor = match statement.execute(None::<&gio::Cancellable>) {
                    Ok(cursor) => cursor,
                    Err(e) => {
                        warn!("Failed to execute query: {}", e);
                        continue;
                    }
                };

                while let Ok(true) = calendar_cursor.next(None::<&gio::Cancellable>) {
                    let calendar_uri = calendar_cursor.string(0).unwrap();
                    let calendar_color = match calendar_cursor.string(1).unwrap().parse() {
                        Ok(color) => color,
                        Err(e) => {
                            warn!("Failed to parse calendar color: {}", e);
                            continue;
                        }
                    };
                    let calendar_name = calendar_cursor.string(2).unwrap();

                    let calendar =
                        Calendar::new(&self.obj(), &calendar_uri, &calendar_name, calendar_color);
                    self.resource_pool().insert(
                        calendar_uri.to_string(),
                        Resource::Calendar(calendar.clone()),
                    );
                    collection.add_calendar(&calendar);

                    info!("Found calendar: uri: \"{calendar_uri}\", name: \"{calendar_name}\"");
                }
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
                let provider = Provider::new(&pre_provider.name);
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
                    let collection =
                        Collection::new(&self.obj(), &pre_collection.uri, &pre_collection.name);
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
                    let event = Event::new(&pre_event.name);
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
            let _update_events = updated_uris
                .into_iter()
                .map(|uri| {
                    let old = self.resource_pool().get(uri.as_str()).unwrap().to_owned();
                    let new = PreResource::from_uri(self.read_connection(), &uri).unwrap();
                    (uri, old, new)
                })
                .collect::<Vec<_>>();

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
                "AddCalendar",
                Some(&(collection_uri, name, &color.to_string()).to_variant()),
                DBusCallFlags::NONE,
                -1,
                None::<&gio::Cancellable>,
            )
            .unwrap();
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
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

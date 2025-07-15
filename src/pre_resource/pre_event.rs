use gdk::gio;
use tracing::error;
use tsparql::{SparqlConnection, prelude::*};

pub struct PreEvent {
    pub uri: String,
    pub calendar_uri: String,
    pub name: String,
    pub description: String,
}

impl PreEvent {
    /// Retrieves an event resource from a URI.
    ///
    /// # Panics
    ///
    /// This function may panic if the given URI is invalid or does not point to an event resource.
    pub fn from_uri(read_connection: &SparqlConnection, uri: &str) -> Result<Self, ()> {
        let statement = read_connection
            .query_statement(
                "SELECT ?name ?description ?calendar
                WHERE {
                    ~uri a ccm:Event ;
                        ccm:calendar ?calendar ;
                        ccm:eventName ?name ;
                        ccm:eventDescription ?description .
                }",
                None::<&gio::Cancellable>,
            )
            .expect("SPARQL should be valid")
            .expect("SPARQL should be valid");
        statement.bind_string("uri", uri);

        let cursor = match statement.execute(None::<&gio::Cancellable>) {
            Ok(cursor) => cursor,
            Err(err) => {
                error!("Failed to create event: {err:?}");
                return Err(());
            }
        };

        match cursor.next(None::<&gio::Cancellable>) {
            Ok(true) => {
                let event_name = cursor.string(0).expect("Query should return an event name");
                let description = cursor
                    .string(1)
                    .expect("Query should return an event description");
                let calendar_uri = cursor
                    .string(2)
                    .expect("Query should return a calendar URI");
                let calendar = Self {
                    uri: uri.to_string(),
                    calendar_uri: calendar_uri.to_string(),
                    name: event_name.to_string(),
                    description: description.to_string(),
                };

                Ok(calendar)
            }
            Ok(false) => {
                error!("Resource {uri} was created but is not found in database");
                Err(())
            }
            Err(e) => {
                error!("Encountered glib error: {}", e);
                Err(())
            }
        }
    }
}

use gdk::gio;
use tracing::error;
use tsparql::{SparqlConnection, prelude::*};

pub struct PreEvent {
    pub uri: String,
    pub calendar_uri: String,
    pub name: String,
    pub description: String,
    pub all_day: bool,
    pub start: String,
    pub end: String,
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
                "SELECT ?name ?description ?calendar ?all_day ?start ?end
                WHERE {
                    ~uri a ccm:Event ;
                        ccm:calendar ?calendar ;
                        ccm:eventName ?name ;
                        ccm:eventDescription ?description  ;
                        ccm:eventAllDay ?all_day ;
                        ccm:eventStart ?start ;
                        ccm:eventEnd ?end .
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
                let name = cursor
                    .string(0)
                    .expect("Query should return an event name")
                    .to_string();
                let description = cursor
                    .string(1)
                    .expect("Query should return an event description")
                    .to_string();
                let calendar_uri = cursor
                    .string(2)
                    .expect("Query should return a calendar URI")
                    .to_string();
                let all_day = cursor.is_boolean(3);
                let start = cursor
                    .string(4)
                    .expect("Query should return a calendar URI")
                    .to_string();
                let end = cursor
                    .string(5)
                    .expect("Query should return a calendar URI")
                    .to_string();
                let calendar = Self {
                    uri: uri.to_string(),
                    calendar_uri,
                    name,
                    description,
                    all_day,
                    start,
                    end,
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

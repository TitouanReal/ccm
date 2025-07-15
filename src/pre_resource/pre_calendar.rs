use gdk::{RGBA, gio};
use tracing::error;
use tsparql::{SparqlConnection, prelude::*};

pub struct PreCalendar {
    pub uri: String,
    pub collection_uri: String,
    pub name: String,
    pub color: RGBA,
}

impl PreCalendar {
    /// Retrieves a calendar resource from a URI.
    ///
    /// # Panics
    ///
    /// This function may panic if the given URI is invalid or does not point to a calendar resource.
    pub fn from_uri(read_connection: &SparqlConnection, uri: &str) -> Result<Self, ()> {
        let statement = read_connection
            .query_statement(
                "SELECT ?name ?color ?collection
                WHERE {
                    ~uri a ccm:Calendar ;
                        ccm:collection ?collection ;
                        ccm:calendarName ?name ;
                        ccm:color ?color .
                }",
                None::<&gio::Cancellable>,
            )
            .expect("SPARQL should be valid")
            .expect("SPARQL should be valid");
        statement.bind_string("uri", uri);

        let cursor = match statement.execute(None::<&gio::Cancellable>) {
            Ok(cursor) => cursor,
            Err(err) => {
                error!("Failed to create calendar: {err:?}");
                return Err(());
            }
        };

        match cursor.next(None::<&gio::Cancellable>) {
            Ok(true) => {
                let calendar_name = cursor
                    .string(0)
                    .expect("Query should return a calendar name");
                let calendar_color = match cursor
                    .string(1)
                    .expect("Query should return a calendar color")
                    .parse()
                {
                    Ok(color) => color,
                    Err(e) => {
                        error!("Invalid color value for calendar {calendar_name}: {e}");
                        return Err(());
                    }
                };
                let collection_uri = cursor
                    .string(2)
                    .expect("Query should return a collection URI");
                let calendar = Self {
                    uri: uri.to_string(),
                    collection_uri: collection_uri.to_string(),
                    name: calendar_name.to_string(),
                    color: calendar_color,
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

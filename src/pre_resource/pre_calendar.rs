use gtk::{gdk::RGBA, gio};
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
        let cursor = read_connection
            .query(
                &format!(
                    "SELECT ?name ?color ?collection
                    FROM ccm:Calendar
                    WHERE {{
                        \"{uri}\" rdfs:label ?name ;
                            ccm:color ?color ;
                            ccm:collection ?collection .
                    }}",
                ),
                None::<&gio::Cancellable>,
            )
            .unwrap();

        match cursor.next(None::<&gio::Cancellable>) {
            Err(e) => {
                error!("Encountered glib error: {}", e);
                Err(())
            }
            Ok(false) => {
                error!("Resource {uri} was created but is not found in database");
                Err(())
            }
            Ok(true) => {
                let calendar_name = cursor.string(0).unwrap();
                let calendar_color = match cursor.string(1).unwrap().parse() {
                    Ok(color) => color,
                    Err(_) => {
                        error!("Invalid color value for calendar {}", calendar_name);
                        return Err(());
                    }
                };
                let collection_uri = cursor.string(2).unwrap();
                let calendar = Self {
                    uri: uri.to_string(),
                    collection_uri: collection_uri.to_string(),
                    name: calendar_name.to_string(),
                    color: calendar_color,
                };

                Ok(calendar)
            }
        }
    }
}

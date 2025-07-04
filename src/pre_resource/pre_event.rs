use tsparql::SparqlConnection;

pub struct PreEvent {
    pub uri: String,
    pub calendar_uri: String,
    pub name: String,
}

impl PreEvent {
    /// Retrieves an event resource from a URI.
    ///
    /// # Panics
    ///
    /// This function may panic if the given URI is invalid or does not point to an event resource.
    pub fn from_uri(_read_connection: &SparqlConnection, _uri: &str) -> Result<Self, ()> {
        todo!()
    }
}

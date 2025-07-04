use tsparql::SparqlConnection;

pub struct PreCollection {
    pub uri: String,
    pub provider_uri: String,
    pub name: String,
}

impl PreCollection {
    /// Retrieves a collection resource from a URI.
    ///
    /// # Panics
    ///
    /// This function may panic if the given URI is invalid or does not point to a collection resource.
    pub fn from_uri(_read_connection: &SparqlConnection, _uri: &str) -> Result<Self, ()> {
        todo!()
    }
}

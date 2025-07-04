use tsparql::SparqlConnection;

pub struct PreProvider {
    pub uri: String,
    pub name: String,
}

impl PreProvider {
    /// Retrieves a provider resource from a URI.
    ///
    /// # Panics
    ///
    /// This function may panic if the given URI is invalid or does not point to a provider resource.
    pub fn from_uri(_read_connection: &SparqlConnection, _uri: &str) -> Result<Self, ()> {
        todo!()
    }
}

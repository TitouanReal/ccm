use crate::{Calendar, Collection, Event, Provider};

#[derive(Debug, Clone)]
pub enum Resource {
    Provider(Provider),
    Collection(Collection),
    Calendar(Calendar),
    Event(Event),
}

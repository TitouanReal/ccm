use std::{
    cell::{Cell, RefCell},
    fmt,
    str::FromStr,
};

use gdk::{
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "Zoned")]
pub struct Zoned(pub jiff::Zoned);

impl fmt::Display for Zoned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Zoned {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        jiff::Zoned::from_str(s).map(Zoned)
    }
}

impl From<jiff::civil::Date> for Zoned {
    fn from(date: jiff::civil::Date) -> Self {
        Zoned(
            date.in_tz("UTC")
                .expect("UTC should exist in timezone database"),
        )
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::Timeframe)]
    pub struct Timeframe {
        #[property(get, construct_only)]
        all_day: Cell<bool>,
        #[property(get, construct_only)]
        start: RefCell<Zoned>,
        #[property(get, construct_only)]
        end: RefCell<Zoned>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Timeframe {
        const NAME: &'static str = "Timeframe";
        type Type = super::Timeframe;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Timeframe {}
}

glib::wrapper! {
    pub struct Timeframe(ObjectSubclass<imp::Timeframe>);
}

impl Timeframe {
    /// Create a new zoned time frame from its properties.
    pub(crate) fn new(all_day: bool, start: Zoned, end: Zoned) -> Self {
        glib::Object::builder()
            .property("all_day", all_day)
            .property("start", start)
            .property("end", end)
            .build()
    }
}

impl Default for Timeframe {
    fn default() -> Self {
        Self::new(false, Zoned::default(), Zoned::default())
    }
}

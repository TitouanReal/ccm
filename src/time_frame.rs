use std::{
    cell::{Cell, RefCell},
    str::FromStr,
};

use gdk::{
    glib::{self, Object},
    prelude::*,
    subclass::prelude::*,
};
use jiff::{Zoned, civil::Date};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstantInner {
    Date(Date),
    Zoned(Zoned),
}

impl Default for InstantInner {
    fn default() -> Self {
        InstantInner::Date(Date::default())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "Instant")]
pub struct Instant(pub InstantInner);

impl Instant {
    pub fn new_date(date: Date) -> Self {
        Instant(InstantInner::Date(date))
    }

    pub fn new_zoned(zoned: Zoned) -> Self {
        Instant(InstantInner::Zoned(zoned))
    }
}

impl FromStr for Instant {
    type Err = jiff::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<Zoned>() {
            Ok(zoned) => Ok(Instant::new_zoned(zoned)),
            Err(_) => match s.parse::<Date>() {
                Ok(date) => Ok(Instant::new_date(date)),
                Err(e) => Err(e),
            },
        }
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::TimeFrame)]
    pub struct TimeFrame {
        #[property(get, construct_only)]
        all_day: Cell<bool>,
        #[property(get, construct_only)]
        start: RefCell<Instant>,
        #[property(get, construct_only)]
        end: RefCell<Instant>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TimeFrame {
        const NAME: &'static str = "Timeframe";
        type Type = super::TimeFrame;
        type ParentType = Object;
    }

    #[glib::derived_properties]
    impl ObjectImpl for TimeFrame {}
}

glib::wrapper! {
    pub struct TimeFrame(ObjectSubclass<imp::TimeFrame>);
}

impl TimeFrame {
    /// Create a new zoned time frame from its properties.
    pub(crate) fn new_zoned(all_day: bool, start: Zoned, end: Zoned) -> Self {
        glib::Object::builder()
            .property("all_day", all_day)
            .property("start", Instant::new_zoned(start))
            .property("end", Instant::new_zoned(end))
            .build()
    }

    /// Create a new date time frame from its properties.
    pub(crate) fn new_date(all_day: bool, start: Date, end: Date) -> Self {
        glib::Object::builder()
            .property("all_day", all_day)
            .property("start", Instant::new_date(start))
            .property("end", Instant::new_date(end))
            .build()
    }
}

impl Default for TimeFrame {
    fn default() -> Self {
        Self::new_zoned(false, Zoned::default(), Zoned::default())
    }
}

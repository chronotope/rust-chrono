use core::{convert::TryFrom, fmt};

#[cfg(feature = "rkyv")]
use rkyv::{Archive, Deserialize, Serialize};

use crate::OutOfRange;

/// The month of the year.
///
/// This enum is just a convenience implementation.
/// The month in dates created by DateLike objects does not return this enum.
///
/// It is possible to convert from a date to a month independently
/// ```
/// # use std::convert::TryFrom;
/// use chrono::prelude::*;
/// let date = Utc.ymd_opt(2019, 10, 28).unwrap().and_hms_opt(9, 10, 11).unwrap();
/// // `2019-10-28T09:10:11Z`
/// let month = Month::try_from(u8::try_from(date.month()).unwrap()).ok();
/// assert_eq!(month, Some(Month::October))
/// ```
/// Or from a Month to an integer usable by dates
/// ```
/// # use chrono::prelude::*;
/// let month = Month::January;
/// let dt = Utc.ymd_opt(2019, month.number_from_month(), 28).unwrap().and_hms_opt(9, 10, 11).unwrap();
/// assert_eq!((dt.year(), dt.month(), dt.day()), (2019, 1, 28));
/// ```
/// Allows mapping from and to month, from 1-January to 12-December.
/// Can be Serialized/Deserialized with serde
// Actual implementation is zero-indexed, API intended as 1-indexed for more intuitive behavior.
#[derive(PartialEq, Eq, Copy, Clone, Debug, Hash)]
#[cfg_attr(feature = "rkyv", derive(Archive, Deserialize, Serialize))]
pub enum Month {
    /// January
    January = 0,
    /// February
    February = 1,
    /// March
    March = 2,
    /// April
    April = 3,
    /// May
    May = 4,
    /// June
    June = 5,
    /// July
    July = 6,
    /// August
    August = 7,
    /// September
    September = 8,
    /// October
    October = 9,
    /// November
    November = 10,
    /// December
    December = 11,
}

impl Month {
    /// The next month.
    ///
    /// `m`:        | `January`  | `February` | `...` | `December`
    /// ----------- | ---------  | ---------- | --- | ---------
    /// `m.succ()`: | `February` | `March`    | `...` | `January`
    #[inline]
    pub fn succ(&self) -> Month {
        match *self {
            Month::January => Month::February,
            Month::February => Month::March,
            Month::March => Month::April,
            Month::April => Month::May,
            Month::May => Month::June,
            Month::June => Month::July,
            Month::July => Month::August,
            Month::August => Month::September,
            Month::September => Month::October,
            Month::October => Month::November,
            Month::November => Month::December,
            Month::December => Month::January,
        }
    }

    /// The previous month.
    ///
    /// `m`:        | `January`  | `February` | `...` | `December`
    /// ----------- | ---------  | ---------- | --- | ---------
    /// `m.pred()`: | `December` | `January`  | `...` | `November`
    #[inline]
    pub fn pred(&self) -> Month {
        match *self {
            Month::January => Month::December,
            Month::February => Month::January,
            Month::March => Month::February,
            Month::April => Month::March,
            Month::May => Month::April,
            Month::June => Month::May,
            Month::July => Month::June,
            Month::August => Month::July,
            Month::September => Month::August,
            Month::October => Month::September,
            Month::November => Month::October,
            Month::December => Month::November,
        }
    }

    /// Returns a month-of-year number starting from January = 1.
    ///
    /// `m`:                     | `January` | `February` | `...` | `December`
    /// -------------------------| --------- | ---------- | --- | -----
    /// `m.number_from_month()`: | 1         | 2          | `...` | 12
    #[inline]
    pub const fn number_from_month(&self) -> u8 {
        match *self {
            Month::January => 1,
            Month::February => 2,
            Month::March => 3,
            Month::April => 4,
            Month::May => 5,
            Month::June => 6,
            Month::July => 7,
            Month::August => 8,
            Month::September => 9,
            Month::October => 10,
            Month::November => 11,
            Month::December => 12,
        }
    }

    /// Get the name of the month
    ///
    /// ```
    /// use chrono::Month;
    ///
    /// assert_eq!(Month::January.name(), "January")
    /// ```
    pub fn name(&self) -> &'static str {
        match *self {
            Month::January => "January",
            Month::February => "February",
            Month::March => "March",
            Month::April => "April",
            Month::May => "May",
            Month::June => "June",
            Month::July => "July",
            Month::August => "August",
            Month::September => "September",
            Month::October => "October",
            Month::November => "November",
            Month::December => "December",
        }
    }

    /// a
    pub const fn try_from_u8(val: u8) -> Option<Self> {
        match val {
            1 => Some(Month::January),
            2 => Some(Month::February),
            3 => Some(Month::March),
            4 => Some(Month::April),
            5 => Some(Month::May),
            6 => Some(Month::June),
            7 => Some(Month::July),
            8 => Some(Month::August),
            9 => Some(Month::September),
            10 => Some(Month::October),
            11 => Some(Month::November),
            12 => Some(Month::December),
            _ => None,
        }
    }

    /// a
    pub const fn try_from_u16(val: u16) -> Option<Self> {
        match val {
            1 => Some(Month::January),
            2 => Some(Month::February),
            3 => Some(Month::March),
            4 => Some(Month::April),
            5 => Some(Month::May),
            6 => Some(Month::June),
            7 => Some(Month::July),
            8 => Some(Month::August),
            9 => Some(Month::September),
            10 => Some(Month::October),
            11 => Some(Month::November),
            12 => Some(Month::December),
            _ => None,
        }
    }

    /// a
    pub const fn try_from_i16(val: i16) -> Option<Self> {
        match val {
            1 => Some(Month::January),
            2 => Some(Month::February),
            3 => Some(Month::March),
            4 => Some(Month::April),
            5 => Some(Month::May),
            6 => Some(Month::June),
            7 => Some(Month::July),
            8 => Some(Month::August),
            9 => Some(Month::September),
            10 => Some(Month::October),
            11 => Some(Month::November),
            12 => Some(Month::December),
            _ => None,
        }
    }
}

impl TryFrom<u8> for Month {
    type Error = OutOfRange;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Month::January),
            2 => Ok(Month::February),
            3 => Ok(Month::March),
            4 => Ok(Month::April),
            5 => Ok(Month::May),
            6 => Ok(Month::June),
            7 => Ok(Month::July),
            8 => Ok(Month::August),
            9 => Ok(Month::September),
            10 => Ok(Month::October),
            11 => Ok(Month::November),
            12 => Ok(Month::December),
            _ => Err(OutOfRange::new()),
        }
    }
}

/// A duration in calendar months
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub struct Months(pub(crate) u32);

impl Months {
    /// Construct a new `Months` from a number of months
    pub fn new(num: u32) -> Self {
        Self(num)
    }
}

/// An error resulting from reading `<Month>` value with `FromStr`.
#[derive(Clone, PartialEq, Eq)]
pub struct ParseMonthError {
    pub(crate) _dummy: (),
}

impl fmt::Debug for ParseMonthError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ParseMonthError {{ .. }}")
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
mod month_serde {
    use super::Month;
    use serde::{de, ser};

    use core::fmt;

    impl ser::Serialize for Month {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            serializer.collect_str(self.name())
        }
    }

    struct MonthVisitor;

    impl<'de> de::Visitor<'de> for MonthVisitor {
        type Value = Month;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("Month")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(|_| E::custom("short (3-letter) or full month names expected"))
        }
    }

    impl<'de> de::Deserialize<'de> for Month {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: de::Deserializer<'de>,
        {
            deserializer.deserialize_str(MonthVisitor)
        }
    }

    #[test]
    fn test_serde_serialize() {
        use serde_json::to_string;
        use Month::*;

        let cases: Vec<(Month, &str)> = vec![
            (January, "\"January\""),
            (February, "\"February\""),
            (March, "\"March\""),
            (April, "\"April\""),
            (May, "\"May\""),
            (June, "\"June\""),
            (July, "\"July\""),
            (August, "\"August\""),
            (September, "\"September\""),
            (October, "\"October\""),
            (November, "\"November\""),
            (December, "\"December\""),
        ];

        for (month, expected_str) in cases {
            let string = to_string(&month).unwrap();
            assert_eq!(string, expected_str);
        }
    }

    #[test]
    fn test_serde_deserialize() {
        use serde_json::from_str;
        use Month::*;

        let cases: Vec<(&str, Month)> = vec![
            ("\"january\"", January),
            ("\"jan\"", January),
            ("\"FeB\"", February),
            ("\"MAR\"", March),
            ("\"mar\"", March),
            ("\"april\"", April),
            ("\"may\"", May),
            ("\"june\"", June),
            ("\"JULY\"", July),
            ("\"august\"", August),
            ("\"september\"", September),
            ("\"October\"", October),
            ("\"November\"", November),
            ("\"DECEmbEr\"", December),
        ];

        for (string, expected_month) in cases {
            let month = from_str::<Month>(string).unwrap();
            assert_eq!(month, expected_month);
        }

        let errors: Vec<&str> =
            vec!["\"not a month\"", "\"ja\"", "\"Dece\"", "Dec", "\"Augustin\""];

        for string in errors {
            from_str::<Month>(string).unwrap_err();
        }
    }
}

#[cfg(test)]
mod tests {
    use core::convert::TryFrom;

    use super::Month;
    use crate::{Datelike, OutOfRange, TimeZone, Utc};

    #[test]
    fn test_month_enum_try_from() {
        assert_eq!(Month::try_from(1), Ok(Month::January));
        assert_eq!(Month::try_from(2), Ok(Month::February));
        assert_eq!(Month::try_from(12), Ok(Month::December));
        assert_eq!(Month::try_from(13), Err(OutOfRange::new()));

        let date = Utc.ymd_opt(2019, 10, 28).unwrap().and_hms_opt(9, 10, 11).unwrap();
        assert_eq!(Month::try_from(date.month() as u8), Ok(Month::October));

        let month = Month::January;
        let dt = Utc
            .ymd_opt(2019, month.number_from_month(), 28)
            .unwrap()
            .and_hms_opt(9, 10, 11)
            .unwrap();
        assert_eq!((dt.year(), dt.month(), dt.day()), (2019, 1, 28));
    }

    #[test]
    fn test_month_enum_succ_pred() {
        assert_eq!(Month::January.succ(), Month::February);
        assert_eq!(Month::December.succ(), Month::January);
        assert_eq!(Month::January.pred(), Month::December);
        assert_eq!(Month::February.pred(), Month::January);
    }
}

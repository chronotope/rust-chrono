// This is a part of Chrono.
// See README.md and LICENSE.txt for details.

//! ISO 8601 date and time without timezone.

#[cfg(feature = "alloc")]
use core::borrow::Borrow;
use core::fmt::Write;
use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::time::Duration;
use core::{fmt, str};

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize, Serialize};

#[cfg(feature = "alloc")]
use crate::format::DelayedFormat;
use crate::format::{parse, parse_and_remainder, ParseError, ParseResult, Parsed, StrftimeItems};
use crate::format::{Fixed, Item, Numeric, Pad};
use crate::naive::{Days, IsoWeek, NaiveDate, NaiveTime};
use crate::offset::Utc;
use crate::{
    expect, ok, try_err, try_ok_or, try_opt, DateTime, Datelike, Error, FixedOffset,
    MappedLocalTime, Months, TimeDelta, TimeZone, Timelike, Weekday,
};

/// Tools to help serializing/deserializing `NaiveDateTime`s
#[cfg(feature = "serde")]
pub(crate) mod serde;

#[cfg(test)]
mod tests;

/// ISO 8601 combined date and time without timezone.
///
/// # Example
///
/// `NaiveDateTime` is commonly created from [`NaiveDate`].
///
/// ```
/// use chrono::{NaiveDate, NaiveDateTime};
///
/// let dt: NaiveDateTime = NaiveDate::from_ymd(2016, 7, 8).unwrap().and_hms(9, 10, 11).unwrap();
/// # let _ = dt;
/// ```
///
/// You can use typical [date-like](Datelike) and [time-like](Timelike) methods,
/// provided that relevant traits are in the scope.
///
/// ```
/// # use chrono::{NaiveDate, NaiveDateTime};
/// # let dt: NaiveDateTime = NaiveDate::from_ymd(2016, 7, 8).unwrap().and_hms(9, 10, 11).unwrap();
/// use chrono::{Datelike, Timelike, Weekday};
///
/// assert_eq!(dt.weekday(), Weekday::Fri);
/// assert_eq!(dt.num_seconds_from_midnight(), 33011);
/// ```
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, Deserialize, Serialize),
    archive(compare(PartialEq, PartialOrd)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
#[cfg_attr(all(feature = "arbitrary", feature = "std"), derive(arbitrary::Arbitrary))]
pub struct NaiveDateTime {
    date: NaiveDate,
    time: NaiveTime,
}

impl NaiveDateTime {
    /// Makes a new `NaiveDateTime` from date and time components.
    /// Equivalent to [`date.and_time(time)`](./struct.NaiveDate.html#method.and_time)
    /// and many other helper constructors on `NaiveDate`.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    ///
    /// let d = NaiveDate::from_ymd(2015, 6, 3).unwrap();
    /// let t = NaiveTime::from_hms_milli(12, 34, 56, 789).unwrap();
    ///
    /// let dt = NaiveDateTime::new(d, t);
    /// assert_eq!(dt.date(), d);
    /// assert_eq!(dt.time(), t);
    /// ```
    #[inline]
    pub const fn new(date: NaiveDate, time: NaiveTime) -> NaiveDateTime {
        NaiveDateTime { date, time }
    }

    /// Parses a string with the specified format string and returns a new `NaiveDateTime`.
    /// See the [`format::strftime` module](crate::format::strftime)
    /// on the supported escape sequences.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, NaiveDateTime};
    ///
    /// let parse_from_str = NaiveDateTime::parse_from_str;
    ///
    /// assert_eq!(
    ///     parse_from_str("2015-09-05 23:56:04", "%Y-%m-%d %H:%M:%S"),
    ///     Ok(NaiveDate::from_ymd(2015, 9, 5).unwrap().and_hms(23, 56, 4).unwrap())
    /// );
    /// assert_eq!(
    ///     parse_from_str("5sep2015pm012345.6789", "%d%b%Y%p%I%M%S%.f"),
    ///     Ok(NaiveDate::from_ymd(2015, 9, 5).unwrap().and_hms_micro(13, 23, 45, 678_900).unwrap())
    /// );
    /// ```
    ///
    /// Offset is ignored for the purpose of parsing.
    ///
    /// ```
    /// # use chrono::{NaiveDateTime, NaiveDate};
    /// # let parse_from_str = NaiveDateTime::parse_from_str;
    /// assert_eq!(
    ///     parse_from_str("2014-5-17T12:34:56+09:30", "%Y-%m-%dT%H:%M:%S%z"),
    ///     Ok(NaiveDate::from_ymd(2014, 5, 17).unwrap().and_hms(12, 34, 56).unwrap())
    /// );
    /// ```
    ///
    /// [Leap seconds](./struct.NaiveTime.html#leap-second-handling) are correctly handled by
    /// treating any time of the form `hh:mm:60` as a leap second.
    /// (This equally applies to the formatting, so the round trip is possible.)
    ///
    /// ```
    /// # use chrono::{NaiveDateTime, NaiveDate};
    /// # let parse_from_str = NaiveDateTime::parse_from_str;
    /// assert_eq!(
    ///     parse_from_str("2015-07-01 08:59:60.123", "%Y-%m-%d %H:%M:%S%.f"),
    ///     Ok(NaiveDate::from_ymd(2015, 7, 1).unwrap().and_hms_milli(8, 59, 59, 1_123).unwrap())
    /// );
    /// ```
    ///
    /// Missing seconds are assumed to be zero,
    /// but out-of-bound times or insufficient fields are errors otherwise.
    ///
    /// ```
    /// # use chrono::{NaiveDateTime, NaiveDate};
    /// # let parse_from_str = NaiveDateTime::parse_from_str;
    /// assert_eq!(
    ///     parse_from_str("94/9/4 7:15", "%y/%m/%d %H:%M"),
    ///     Ok(NaiveDate::from_ymd(1994, 9, 4).unwrap().and_hms(7, 15, 0).unwrap())
    /// );
    ///
    /// assert!(parse_from_str("04m33s", "%Mm%Ss").is_err());
    /// assert!(parse_from_str("94/9/4 12", "%y/%m/%d %H").is_err());
    /// assert!(parse_from_str("94/9/4 17:60", "%y/%m/%d %H:%M").is_err());
    /// assert!(parse_from_str("94/9/4 24:00:00", "%y/%m/%d %H:%M:%S").is_err());
    /// ```
    ///
    /// All parsed fields should be consistent to each other, otherwise it's an error.
    ///
    /// ```
    /// # use chrono::NaiveDateTime;
    /// # let parse_from_str = NaiveDateTime::parse_from_str;
    /// let fmt = "%Y-%m-%d %H:%M:%S = UNIX timestamp %s";
    /// assert!(parse_from_str("2001-09-09 01:46:39 = UNIX timestamp 999999999", fmt).is_ok());
    /// assert!(parse_from_str("1970-01-01 00:00:00 = UNIX timestamp 1", fmt).is_err());
    /// ```
    ///
    /// Years before 1 BCE or after 9999 CE, require an initial sign
    ///
    ///```
    /// # use chrono::NaiveDateTime;
    /// # let parse_from_str = NaiveDateTime::parse_from_str;
    /// let fmt = "%Y-%m-%d %H:%M:%S";
    /// assert!(parse_from_str("10000-09-09 01:46:39", fmt).is_err());
    /// assert!(parse_from_str("+10000-09-09 01:46:39", fmt).is_ok());
    /// ```
    pub fn parse_from_str(s: &str, fmt: &str) -> ParseResult<NaiveDateTime> {
        let mut parsed = Parsed::default();
        parse(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_naive_datetime_with_offset(0) // no offset adjustment
    }

    /// Parses a string with the specified format string and returns a new `NaiveDateTime`, and a
    /// slice with the remaining portion of the string.
    /// See the [`format::strftime` module](crate::format::strftime)
    /// on the supported escape sequences.
    ///
    /// Similar to [`parse_from_str`](#method.parse_from_str).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use chrono::{NaiveDate, NaiveDateTime};
    /// let (datetime, remainder) = NaiveDateTime::parse_and_remainder(
    ///     "2015-02-18 23:16:09 trailing text",
    ///     "%Y-%m-%d %H:%M:%S",
    /// )
    /// .unwrap();
    /// assert_eq!(datetime, NaiveDate::from_ymd(2015, 2, 18).unwrap().and_hms(23, 16, 9).unwrap());
    /// assert_eq!(remainder, " trailing text");
    /// ```
    pub fn parse_and_remainder<'a>(s: &'a str, fmt: &str) -> ParseResult<(NaiveDateTime, &'a str)> {
        let mut parsed = Parsed::default();
        let remainder = parse_and_remainder(&mut parsed, s, StrftimeItems::new(fmt))?;
        parsed.to_naive_datetime_with_offset(0).map(|d| (d, remainder)) // no offset adjustment
    }

    /// Retrieves a date component.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::NaiveDate;
    ///
    /// let dt = NaiveDate::from_ymd(2016, 7, 8).unwrap().and_hms(9, 10, 11).unwrap();
    /// assert_eq!(dt.date(), NaiveDate::from_ymd(2016, 7, 8).unwrap());
    /// ```
    #[inline]
    pub const fn date(self) -> NaiveDate {
        self.date
    }

    /// Retrieves a time component.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, NaiveTime};
    ///
    /// let dt = NaiveDate::from_ymd(2016, 7, 8).unwrap().and_hms(9, 10, 11).unwrap();
    /// assert_eq!(dt.time(), NaiveTime::from_hms(9, 10, 11).unwrap());
    /// ```
    #[inline]
    pub const fn time(self) -> NaiveTime {
        self.time
    }

    /// Adds given `TimeDelta` to the current date and time.
    ///
    /// As a part of Chrono's [leap second handling](./struct.NaiveTime.html#leap-second-handling),
    /// the addition assumes that **there is no leap second ever**,
    /// except when the `NaiveDateTime` itself represents a leap second
    /// in which case the assumption becomes that **there is exactly a single leap second ever**.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the resulting date would be out of range.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, TimeDelta};
    ///
    /// let d = NaiveDate::from_ymd(2016, 7, 8)?;
    /// let hms = |h, m, s| d.and_hms(h, m, s);
    /// assert_eq!(hms(3, 5, 7)?.checked_add_signed(TimeDelta::zero()), hms(3, 5, 7));
    /// assert_eq!(hms(3, 5, 7)?.checked_add_signed(TimeDelta::seconds(1)), hms(3, 5, 8));
    /// assert_eq!(hms(3, 5, 7)?.checked_add_signed(TimeDelta::seconds(-1)), hms(3, 5, 6));
    /// assert_eq!(hms(3, 5, 7)?.checked_add_signed(TimeDelta::seconds(3600 + 60)), hms(4, 6, 7));
    /// assert_eq!(
    ///     hms(3, 5, 7)?.checked_add_signed(TimeDelta::seconds(86_400)),
    ///     NaiveDate::from_ymd(2016, 7, 9)?.and_hms(3, 5, 7)
    /// );
    ///
    /// let hmsm = |h, m, s, milli| d.and_hms_milli(h, m, s, milli);
    /// assert_eq!(
    ///     hmsm(3, 5, 7, 980)?.checked_add_signed(TimeDelta::milliseconds(450).unwrap()),
    ///     hmsm(3, 5, 8, 430)
    /// );
    /// # Ok::<(), chrono::Error>(())
    /// ```
    ///
    /// Overflow returns [`Error::OutOfRange`].
    ///
    /// ```
    /// # use chrono::{Error, NaiveDate, TimeDelta};
    /// # let d = NaiveDate::from_ymd(2016, 7, 8)?;
    /// # let hms = |h, m, s| d.and_hms(h, m, s);
    /// assert_eq!(
    ///     hms(3, 5, 7)?.checked_add_signed(TimeDelta::days(1_000_000_000)),
    ///     Err(Error::OutOfRange)
    /// );
    /// # Ok::<(), Error>(())
    /// ```
    ///
    /// Leap seconds are handled,
    /// but the addition assumes that it is the only leap second happened.
    ///
    /// ```
    /// # use chrono::{NaiveDate, TimeDelta};
    /// # let d = NaiveDate::from_ymd(2016, 7, 8)?;
    /// # let hmsm = |h, m, s, milli| d.and_hms_milli(h, m, s, milli);
    /// let leap = hmsm(3, 5, 59, 1_300)?;
    /// assert_eq!(leap.checked_add_signed(TimeDelta::zero()), hmsm(3, 5, 59, 1_300));
    /// assert_eq!(
    ///     leap.checked_add_signed(TimeDelta::milliseconds(-500).unwrap()),
    ///     hmsm(3, 5, 59, 800)
    /// );
    /// assert_eq!(
    ///     leap.checked_add_signed(TimeDelta::milliseconds(500).unwrap()),
    ///     hmsm(3, 5, 59, 1_800)
    /// );
    /// assert_eq!(leap.checked_add_signed(TimeDelta::milliseconds(800).unwrap()), hmsm(3, 6, 0, 100));
    /// assert_eq!(leap.checked_add_signed(TimeDelta::seconds(10)), hmsm(3, 6, 9, 300));
    /// assert_eq!(leap.checked_add_signed(TimeDelta::seconds(-10)), hmsm(3, 5, 50, 300));
    /// assert_eq!(
    ///     leap.checked_add_signed(TimeDelta::days(1)),
    ///     NaiveDate::from_ymd(2016, 7, 9)?.and_hms_milli(3, 5, 59, 300)
    /// );
    /// # Ok::<(), chrono::Error>(())
    /// ```
    pub const fn checked_add_signed(self, rhs: TimeDelta) -> Result<NaiveDateTime, Error> {
        let (time, remainder) = self.time.overflowing_add_signed(rhs);
        let remainder = try_ok_or!(TimeDelta::new(remainder, 0), Error::OutOfRange);
        let date = try_err!(self.date.checked_add_signed(remainder));
        Ok(NaiveDateTime { date, time })
    }

    /// Adds given `Months` to the current date and time.
    ///
    /// Uses the last day of the month if the day does not exist in the resulting month.
    ///
    /// # Errors
    ///
    /// Returns `None` if the resulting date would be out of range.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Months, NaiveDate};
    ///
    /// assert_eq!(
    ///     NaiveDate::from_ymd(2014, 1, 1)
    ///         .unwrap()
    ///         .and_hms(1, 0, 0)
    ///         .unwrap()
    ///         .checked_add_months(Months::new(1)),
    ///     Some(NaiveDate::from_ymd(2014, 2, 1).unwrap().and_hms(1, 0, 0).unwrap())
    /// );
    ///
    /// assert_eq!(
    ///     NaiveDate::from_ymd(2014, 1, 1)
    ///         .unwrap()
    ///         .and_hms(1, 0, 0)
    ///         .unwrap()
    ///         .checked_add_months(Months::new(core::i32::MAX as u32 + 1)),
    ///     None
    /// );
    /// ```
    #[must_use]
    pub const fn checked_add_months(self, rhs: Months) -> Option<NaiveDateTime> {
        Some(Self { date: try_opt!(self.date.checked_add_months(rhs)), time: self.time })
    }

    /// Adds given `FixedOffset` to the current datetime.
    ///
    /// This method is similar to [`checked_add_signed`](#method.checked_add_offset), but preserves
    /// leap seconds.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the result would be outside the valid range for
    /// [`NaiveDateTime`].
    pub const fn checked_add_offset(self, rhs: FixedOffset) -> Result<NaiveDateTime, Error> {
        let (time, days) = self.time.overflowing_add_offset(rhs);
        let date = match days {
            -1 => try_err!(self.date.pred()),
            1 => try_err!(self.date.succ()),
            _ => self.date,
        };
        Ok(NaiveDateTime { date, time })
    }

    /// Subtracts given `FixedOffset` from the current datetime.
    ///
    /// This method is similar to [`checked_sub_signed`](#method.checked_sub_signed), but preserves
    /// leap seconds.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`] if the result would be outside the valid range for
    /// [`NaiveDateTime`].
    pub const fn checked_sub_offset(self, rhs: FixedOffset) -> Result<NaiveDateTime, Error> {
        let (time, days) = self.time.overflowing_sub_offset(rhs);
        let date = match days {
            -1 => try_err!(self.date.pred()),
            1 => try_err!(self.date.succ()),
            _ => self.date,
        };
        Ok(NaiveDateTime { date, time })
    }

    /// Adds given `FixedOffset` to the current datetime.
    /// The resulting value may be outside the valid range of [`NaiveDateTime`].
    ///
    /// This can be useful for intermediate values, but the resulting out-of-range `NaiveDate`
    /// should not be exposed to library users.
    #[must_use]
    pub(crate) fn overflowing_add_offset(self, rhs: FixedOffset) -> NaiveDateTime {
        let (time, days) = self.time.overflowing_add_offset(rhs);
        let date = match days {
            -1 => self.date.pred().unwrap_or(NaiveDate::BEFORE_MIN),
            1 => self.date.succ().unwrap_or(NaiveDate::AFTER_MAX),
            _ => self.date,
        };
        NaiveDateTime { date, time }
    }

    /// Subtracts given `FixedOffset` from the current datetime.
    /// The resulting value may be outside the valid range of [`NaiveDateTime`].
    ///
    /// This can be useful for intermediate values, but the resulting out-of-range `NaiveDate`
    /// should not be exposed to library users.
    #[must_use]
    #[allow(unused)] // currently only used in `Local` but not on all platforms
    pub(crate) fn overflowing_sub_offset(self, rhs: FixedOffset) -> NaiveDateTime {
        let (time, days) = self.time.overflowing_sub_offset(rhs);
        let date = match days {
            -1 => self.date.pred().unwrap_or(NaiveDate::BEFORE_MIN),
            1 => self.date.succ().unwrap_or(NaiveDate::AFTER_MAX),
            _ => self.date,
        };
        NaiveDateTime { date, time }
    }

    /// Subtracts given `TimeDelta` from the current date and time.
    ///
    /// As a part of Chrono's [leap second handling](./struct.NaiveTime.html#leap-second-handling),
    /// the subtraction assumes that **there is no leap second ever**,
    /// except when the `NaiveDateTime` itself represents a leap second
    /// in which case the assumption becomes that **there is exactly a single leap second ever**.
    ///
    /// # Errors
    ///
    /// Returns `[`Error::OutOfRange`] if the resulting date would be out of range.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, TimeDelta};
    ///
    /// let d = NaiveDate::from_ymd(2016, 7, 8)?;
    /// let hms = |h, m, s| d.and_hms(h, m, s);
    /// assert_eq!(hms(3, 5, 7)?.checked_sub_signed(TimeDelta::zero()), hms(3, 5, 7));
    /// assert_eq!(hms(3, 5, 7)?.checked_sub_signed(TimeDelta::seconds(1)), hms(3, 5, 6));
    /// assert_eq!(hms(3, 5, 7)?.checked_sub_signed(TimeDelta::seconds(-1)), hms(3, 5, 8));
    /// assert_eq!(hms(3, 5, 7)?.checked_sub_signed(TimeDelta::seconds(3600 + 60)), hms(2, 4, 7));
    /// assert_eq!(
    ///     hms(3, 5, 7)?.checked_sub_signed(TimeDelta::seconds(86_400)),
    ///     NaiveDate::from_ymd(2016, 7, 7)?.and_hms(3, 5, 7)
    /// );
    ///
    /// let hmsm = |h, m, s, milli| d.and_hms_milli(h, m, s, milli);
    /// assert_eq!(
    ///     hmsm(3, 5, 7, 450)?.checked_sub_signed(TimeDelta::milliseconds(670).unwrap()),
    ///     hmsm(3, 5, 6, 780)
    /// );
    /// # Ok::<(), chrono::Error>(())
    /// ```
    ///
    /// Overflow returns [`Error::OutOfRange`].
    ///
    /// ```
    /// # use chrono::{Error, NaiveDate, TimeDelta};
    /// # let d = NaiveDate::from_ymd(2016, 7, 8)?;
    /// # let hms = |h, m, s| d.and_hms(h, m, s);
    /// assert_eq!(
    ///     hms(3, 5, 7)?.checked_sub_signed(TimeDelta::days(1_000_000_000)),
    ///     Err(Error::OutOfRange)
    /// );
    /// # Ok::<(), Error>(())
    /// ```
    ///
    /// Leap seconds are handled,
    /// but the subtraction assumes that it is the only leap second happened.
    ///
    /// ```
    /// # use chrono::{NaiveDate, TimeDelta};
    /// # let d = NaiveDate::from_ymd(2016, 7, 8)?;
    /// # let hmsm = |h, m, s, milli| d.and_hms_milli(h, m, s, milli);
    /// let leap = hmsm(3, 5, 59, 1_300)?;
    /// assert_eq!(leap.checked_sub_signed(TimeDelta::zero()), hmsm(3, 5, 59, 1_300));
    /// assert_eq!(
    ///     leap.checked_sub_signed(TimeDelta::milliseconds(200).unwrap()),
    ///     hmsm(3, 5, 59, 1_100)
    /// );
    /// assert_eq!(leap.checked_sub_signed(TimeDelta::milliseconds(500).unwrap()), hmsm(3, 5, 59, 800));
    /// assert_eq!(leap.checked_sub_signed(TimeDelta::seconds(60)), hmsm(3, 5, 0, 300));
    /// assert_eq!(
    ///     leap.checked_sub_signed(TimeDelta::days(1)),
    ///     NaiveDate::from_ymd(2016, 7, 7)?.and_hms_milli(3, 6, 0, 300)
    /// );
    /// # Ok::<(), chrono::Error>(())
    /// ```
    pub const fn checked_sub_signed(self, rhs: TimeDelta) -> Result<NaiveDateTime, Error> {
        let (time, remainder) = self.time.overflowing_sub_signed(rhs);
        let remainder = try_ok_or!(TimeDelta::new(remainder, 0), Error::OutOfRange);
        let date = try_err!(self.date.checked_sub_signed(remainder));
        Ok(NaiveDateTime { date, time })
    }

    /// Subtracts given `Months` from the current date and time.
    ///
    /// Uses the last day of the month if the day does not exist in the resulting month.
    ///
    /// # Errors
    ///
    /// Returns `None` if the resulting date would be out of range.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Months, NaiveDate};
    ///
    /// assert_eq!(
    ///     NaiveDate::from_ymd(2014, 1, 1)
    ///         .unwrap()
    ///         .and_hms(1, 0, 0)
    ///         .unwrap()
    ///         .checked_sub_months(Months::new(1)),
    ///     Some(NaiveDate::from_ymd(2013, 12, 1).unwrap().and_hms(1, 0, 0).unwrap())
    /// );
    ///
    /// assert_eq!(
    ///     NaiveDate::from_ymd(2014, 1, 1)
    ///         .unwrap()
    ///         .and_hms(1, 0, 0)
    ///         .unwrap()
    ///         .checked_sub_months(Months::new(core::i32::MAX as u32 + 1)),
    ///     None
    /// );
    /// ```
    #[must_use]
    pub const fn checked_sub_months(self, rhs: Months) -> Option<NaiveDateTime> {
        Some(Self { date: try_opt!(self.date.checked_sub_months(rhs)), time: self.time })
    }

    /// Add a duration in [`Days`] to the date part of the `NaiveDateTime`
    ///
    /// Returns [`Error::OutOfRange`] if the resulting date would be out of range.
    pub const fn checked_add_days(self, days: Days) -> Result<Self, Error> {
        Ok(Self { date: try_err!(self.date.checked_add_days(days)), ..self })
    }

    /// Subtract a duration in [`Days`] from the date part of the `NaiveDateTime`
    ///
    /// Returns [`Error::OutOfRange`] if the resulting date would be out of range.
    pub const fn checked_sub_days(self, days: Days) -> Result<Self, Error> {
        Ok(Self { date: try_err!(self.date.checked_sub_days(days)), ..self })
    }

    /// Subtracts another `NaiveDateTime` from the current date and time.
    /// This does not overflow or underflow at all.
    ///
    /// As a part of Chrono's [leap second handling](./struct.NaiveTime.html#leap-second-handling),
    /// the subtraction assumes that **there is no leap second ever**,
    /// except when any of the `NaiveDateTime`s themselves represents a leap second
    /// in which case the assumption becomes that
    /// **there are exactly one (or two) leap second(s) ever**.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, TimeDelta};
    ///
    /// let from_ymd = |y, m, d| NaiveDate::from_ymd(y, m, d).unwrap();
    ///
    /// let d = from_ymd(2016, 7, 8);
    /// assert_eq!(
    ///     d.and_hms(3, 5, 7).unwrap().signed_duration_since(d.and_hms(2, 4, 6).unwrap()),
    ///     TimeDelta::seconds(3600 + 60 + 1)
    /// );
    ///
    /// // July 8 is 190th day in the year 2016
    /// let d0 = from_ymd(2016, 1, 1);
    /// assert_eq!(
    ///     d.and_hms_milli(0, 7, 6, 500).unwrap().signed_duration_since(d0.and_hms(0, 0, 0).unwrap()),
    ///     TimeDelta::seconds(189 * 86_400 + 7 * 60 + 6) + TimeDelta::milliseconds(500).unwrap()
    /// );
    /// ```
    ///
    /// Leap seconds are handled, but the subtraction assumes that
    /// there were no other leap seconds happened.
    ///
    /// ```
    /// # use chrono::{TimeDelta, NaiveDate};
    /// # let from_ymd = |y, m, d| NaiveDate::from_ymd(y, m, d).unwrap();
    /// let leap = from_ymd(2015, 6, 30).and_hms_milli(23, 59, 59, 1_500).unwrap();
    /// assert_eq!(
    ///     leap.signed_duration_since(from_ymd(2015, 6, 30).and_hms(23, 0, 0).unwrap()),
    ///     TimeDelta::seconds(3600) + TimeDelta::milliseconds(500).unwrap()
    /// );
    /// assert_eq!(
    ///     from_ymd(2015, 7, 1).and_hms(1, 0, 0).unwrap().signed_duration_since(leap),
    ///     TimeDelta::seconds(3600) - TimeDelta::milliseconds(500).unwrap()
    /// );
    /// ```
    #[must_use]
    pub const fn signed_duration_since(self, rhs: NaiveDateTime) -> TimeDelta {
        expect!(
            self.date
                .signed_duration_since(rhs.date)
                .checked_add(self.time.signed_duration_since(rhs.time)),
            "always in range"
        )
    }

    /// Formats the combined date and time with the specified formatting items.
    /// Otherwise it is the same as the ordinary [`format`](#method.format) method.
    ///
    /// The `Iterator` of items should be `Clone`able,
    /// since the resulting `DelayedFormat` value may be formatted multiple times.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::format::strftime::StrftimeItems;
    /// use chrono::NaiveDate;
    ///
    /// let fmt = StrftimeItems::new("%Y-%m-%d %H:%M:%S");
    /// let dt = NaiveDate::from_ymd(2015, 9, 5).unwrap().and_hms(23, 56, 4).unwrap();
    /// assert_eq!(dt.format_with_items(fmt.clone()).to_string(), "2015-09-05 23:56:04");
    /// assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2015-09-05 23:56:04");
    /// ```
    ///
    /// The resulting `DelayedFormat` can be formatted directly via the `Display` trait.
    ///
    /// ```
    /// # use chrono::NaiveDate;
    /// # use chrono::format::strftime::StrftimeItems;
    /// # let fmt = StrftimeItems::new("%Y-%m-%d %H:%M:%S").clone();
    /// # let dt = NaiveDate::from_ymd(2015, 9, 5).unwrap().and_hms(23, 56, 4).unwrap();
    /// assert_eq!(format!("{}", dt.format_with_items(fmt)), "2015-09-05 23:56:04");
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub fn format_with_items<'a, I, B>(self, items: I) -> DelayedFormat<I>
    where
        I: Iterator<Item = B> + Clone,
        B: Borrow<Item<'a>>,
    {
        DelayedFormat::new(Some(self.date), Some(self.time), items)
    }

    /// Formats the combined date and time with the specified format string.
    /// See the [`format::strftime` module](crate::format::strftime)
    /// on the supported escape sequences.
    ///
    /// This returns a `DelayedFormat`,
    /// which gets converted to a string only when actual formatting happens.
    /// You may use the `to_string` method to get a `String`,
    /// or just feed it into `print!` and other formatting macros.
    /// (In this way it avoids the redundant memory allocation.)
    ///
    /// A wrong format string does *not* issue an error immediately.
    /// Rather, converting or formatting the `DelayedFormat` fails.
    /// You are recommended to immediately use `DelayedFormat` for this reason.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::NaiveDate;
    ///
    /// let dt = NaiveDate::from_ymd(2015, 9, 5).unwrap().and_hms(23, 56, 4).unwrap();
    /// assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2015-09-05 23:56:04");
    /// assert_eq!(dt.format("around %l %p on %b %-d").to_string(), "around 11 PM on Sep 5");
    /// ```
    ///
    /// The resulting `DelayedFormat` can be formatted directly via the `Display` trait.
    ///
    /// ```
    /// # use chrono::NaiveDate;
    /// # let dt = NaiveDate::from_ymd(2015, 9, 5).unwrap().and_hms(23, 56, 4).unwrap();
    /// assert_eq!(format!("{}", dt.format("%Y-%m-%d %H:%M:%S")), "2015-09-05 23:56:04");
    /// assert_eq!(format!("{}", dt.format("around %l %p on %b %-d")), "around 11 PM on Sep 5");
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    #[must_use]
    pub fn format(self, fmt: &str) -> DelayedFormat<StrftimeItems> {
        self.format_with_items(StrftimeItems::new(fmt))
    }

    /// Converts the `NaiveDateTime` into the timezone-aware `DateTime<Tz>`
    /// with the provided timezone, if possible.
    ///
    /// This can fail in cases where the local time represented by the `NaiveDateTime`
    /// is not a valid local timestamp in the target timezone due to an offset transition
    /// for example if the target timezone had a change from +00:00 to +01:00
    /// occurring at 2015-09-05 22:59:59, then a local time of 2015-09-05 23:56:04
    /// could never occur. Similarly, if the offset transitioned in the opposite direction
    /// then there would be two local times of 2015-09-05 23:56:04, one at +00:00 and one
    /// at +01:00.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{FixedOffset, NaiveDate};
    /// let hour = 3600;
    /// let tz = FixedOffset::east(5 * hour).unwrap();
    /// let dt = NaiveDate::from_ymd(2015, 9, 5)
    ///     .unwrap()
    ///     .and_hms(23, 56, 4)
    ///     .unwrap()
    ///     .and_local_timezone(tz)
    ///     .unwrap();
    /// assert_eq!(dt.timezone(), tz);
    /// ```
    #[must_use]
    pub fn and_local_timezone<Tz: TimeZone>(self, tz: Tz) -> MappedLocalTime<DateTime<Tz>> {
        tz.from_local_datetime(self)
    }

    /// Converts the `NaiveDateTime` into the timezone-aware `DateTime<Utc>`.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, Utc};
    /// let dt = NaiveDate::from_ymd(2023, 1, 30).unwrap().and_hms(19, 32, 33).unwrap().and_utc();
    /// assert_eq!(dt.timezone(), Utc);
    /// ```
    #[must_use]
    pub const fn and_utc(self) -> DateTime<Utc> {
        DateTime::from_naive_utc_and_offset(self, Utc)
    }

    /// Makes a new `NaiveDateTime` with the year number changed, while keeping the same month and
    /// day.
    ///
    /// See also the [`NaiveDate::with_year`] method.
    ///
    /// # Errors
    ///
    /// Returns:
    /// - [`Error::DoesNotExist`] if the resulting date does not exist.
    /// - [`Error::OutOfRange`] if `year` is out of range for a `NaiveDate`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use chrono::{NaiveDate, NaiveDateTime, Error};
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 25)?.and_hms(12, 34, 56)?;
    /// assert_eq!(dt.with_year(2016), NaiveDate::from_ymd(2016, 9, 25)?.and_hms(12, 34, 56));
    /// assert_eq!(dt.with_year(-308), NaiveDate::from_ymd(-308, 9, 25)?.and_hms(12, 34, 56));
    /// # Ok::<(), Error>(())
    /// ```
    #[inline]
    pub const fn with_year(self, year: i32) -> Result<NaiveDateTime, Error> {
        Ok(NaiveDateTime { date: try_err!(self.date.with_year(year)), ..self })
    }

    /// Makes a new `NaiveDateTime` with the month number (starting from 1) changed.
    ///
    /// Don't combine multiple `Datelike::with_*` methods. The intermediate value may not exist.
    ///
    /// See also the [`NaiveDate::with_month`] method.
    ///
    /// # Errors
    ///
    /// This method returns:
    /// - [`Error::DoesNotExist`] if the resulting date does not exist (for example `month(4)` when
    ///   day of the month is 31).
    /// - [`Error::InvalidArgument`] if the value for `month` is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Error, NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 30)?.and_hms(12, 34, 56)?;
    /// assert_eq!(dt.with_month(10), NaiveDate::from_ymd(2015, 10, 30)?.and_hms(12, 34, 56));
    /// assert_eq!(dt.with_month(13), Err(Error::InvalidArgument));
    /// assert_eq!(dt.with_month(2), Err(Error::DoesNotExist)); // No February 30
    /// # Ok::<(), Error>(())
    /// ```
    #[inline]
    pub const fn with_month(self, month: u32) -> Result<NaiveDateTime, Error> {
        Ok(NaiveDateTime { date: try_err!(self.date.with_month(month)), ..self })
    }

    /// Makes a new `NaiveDateTime` with the day of month (starting from 1) changed.
    ///
    /// See also the [`NaiveDate::with_day`] method.
    ///
    /// # Errors
    ///
    /// This method returns:
    /// - [`Error::DoesNotExist`] if the resulting date does not exist (for example `day(31)` in
    ///   April).
    /// - [`Error::InvalidArgument`] if the value for `day` is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Error, NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 8)?.and_hms(12, 34, 56)?;
    /// assert_eq!(dt.with_day(30), NaiveDate::from_ymd(2015, 9, 30)?.and_hms(12, 34, 56));
    /// assert_eq!(dt.with_day(31), Err(Error::DoesNotExist)); // No September 31
    /// # Ok::<(), Error>(())
    /// ```
    #[inline]
    pub const fn with_day(self, day: u32) -> Result<NaiveDateTime, Error> {
        Ok(NaiveDateTime { date: try_err!(self.date.with_day(day)), ..self })
    }

    /// Makes a new `NaiveDateTime` with the day of year (starting from 1) changed.
    ///
    /// See also the [`NaiveDate::with_ordinal`] method.
    ///
    /// # Errors
    ///
    /// Returns `None` if:
    /// - The resulting date does not exist (`with_ordinal(366)` in a non-leap year).
    /// - The value for `ordinal` is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 8).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(
    ///     dt.with_ordinal(60),
    ///     Some(NaiveDate::from_ymd(2015, 3, 1).unwrap().and_hms(12, 34, 56).unwrap())
    /// );
    /// assert_eq!(dt.with_ordinal(366), None); // 2015 had only 365 days
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2016, 9, 8).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(
    ///     dt.with_ordinal(60),
    ///     Some(NaiveDate::from_ymd(2016, 2, 29).unwrap().and_hms(12, 34, 56).unwrap())
    /// );
    /// assert_eq!(
    ///     dt.with_ordinal(366),
    ///     Some(NaiveDate::from_ymd(2016, 12, 31).unwrap().and_hms(12, 34, 56).unwrap())
    /// );
    /// ```
    #[inline]
    pub const fn with_ordinal(self, ordinal: u32) -> Option<NaiveDateTime> {
        Some(NaiveDateTime { date: try_opt!(self.date.with_ordinal(ordinal)), ..self })
    }

    /// Makes a new `NaiveDateTime` with the hour number changed.
    ///
    /// See also the [`NaiveTime::with_hour`] method.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if the value for `hour` is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Error, NaiveDate};
    ///
    /// let dt = NaiveDate::from_ymd(2015, 9, 8)?.and_hms_milli(12, 34, 56, 789)?;
    /// assert_eq!(dt.with_hour(7), NaiveDate::from_ymd(2015, 9, 8)?.and_hms_milli(7, 34, 56, 789));
    /// assert_eq!(dt.with_hour(24), Err(Error::InvalidArgument));
    /// # Ok::<(), chrono::Error>(())
    /// ```
    #[inline]
    pub const fn with_hour(self, hour: u32) -> Result<NaiveDateTime, Error> {
        Ok(NaiveDateTime { time: try_err!(self.time.with_hour(hour)), ..self })
    }

    /// Makes a new `NaiveDateTime` with the minute number changed.
    ///
    /// See also the [`NaiveTime::with_minute`] method.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if the value for `minute` is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Error, NaiveDate};
    ///
    /// let dt = NaiveDate::from_ymd(2015, 9, 8)?.and_hms_milli(12, 34, 56, 789)?;
    /// assert_eq!(dt.with_minute(45), NaiveDate::from_ymd(2015, 9, 8)?.and_hms_milli(12, 45, 56, 789));
    /// assert_eq!(dt.with_minute(60), Err(Error::InvalidArgument));
    /// # Ok::<(), chrono::Error>(())
    /// ```
    #[inline]
    pub const fn with_minute(self, min: u32) -> Result<NaiveDateTime, Error> {
        Ok(NaiveDateTime { time: try_err!(self.time.with_minute(min)), ..self })
    }

    /// Makes a new `NaiveDateTime` with the second number changed.
    ///
    /// As with the [`second`](#method.second) method,
    /// the input range is restricted to 0 through 59.
    ///
    /// See also the [`NaiveTime::with_second`] method.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if the value for `second` is invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Error, NaiveDate};
    ///
    /// let dt = NaiveDate::from_ymd(2015, 9, 8)?.and_hms_milli(12, 34, 56, 789)?;
    /// assert_eq!(dt.with_second(17), NaiveDate::from_ymd(2015, 9, 8)?.and_hms_milli(12, 34, 17, 789));
    /// assert_eq!(dt.with_second(60), Err(Error::InvalidArgument));
    /// # Ok::<(), chrono::Error>(())
    /// ```
    #[inline]
    pub const fn with_second(self, sec: u32) -> Result<NaiveDateTime, Error> {
        Ok(NaiveDateTime { time: try_err!(self.time.with_second(sec)), ..self })
    }

    /// Makes a new `NaiveDateTime` with nanoseconds since the whole non-leap second changed.
    ///
    /// As with the [`NaiveDateTime::nanosecond`] method, the input range can exceed 1,000,000,000
    /// for leap seconds.
    ///
    /// See also the [`NaiveTime::with_nanosecond`] method.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] if `nanosecond >= 2,000,000,000`.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Error, NaiveDate};
    ///
    /// let dt = NaiveDate::from_ymd(2015, 9, 8)?.and_hms_milli(12, 34, 59, 789)?;
    /// assert_eq!(
    ///     dt.with_nanosecond(333_333_333),
    ///     NaiveDate::from_ymd(2015, 9, 8)?.and_hms_nano(12, 34, 59, 333_333_333)
    /// );
    /// assert_eq!(
    ///     dt.with_nanosecond(1_333_333_333), // leap second
    ///     NaiveDate::from_ymd(2015, 9, 8)?.and_hms_nano(12, 34, 59, 1_333_333_333)
    /// );
    /// assert_eq!(dt.with_nanosecond(2_000_000_000), Err(Error::InvalidArgument));
    /// # Ok::<(), chrono::Error>(())
    /// ```
    #[inline]
    pub const fn with_nanosecond(self, nano: u32) -> Result<NaiveDateTime, Error> {
        Ok(NaiveDateTime { time: try_err!(self.time.with_nanosecond(nano)), ..self })
    }

    /// The minimum possible `NaiveDateTime`.
    pub const MIN: Self = Self { date: NaiveDate::MIN, time: NaiveTime::MIN };

    /// The maximum possible `NaiveDateTime`.
    pub const MAX: Self = Self { date: NaiveDate::MAX, time: NaiveTime::MAX };

    /// The Unix Epoch, 1970-01-01 00:00:00.
    pub const UNIX_EPOCH: Self =
        expect!(ok!(NaiveDate::from_ymd(1970, 1, 1)), "").and_time(NaiveTime::MIN);
}

impl From<NaiveDate> for NaiveDateTime {
    /// Converts a `NaiveDate` to a `NaiveDateTime` of the same date but at midnight.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, NaiveDateTime};
    ///
    /// let nd = NaiveDate::from_ymd(2016, 5, 28).unwrap();
    /// let ndt = NaiveDate::from_ymd(2016, 5, 28).unwrap().and_hms(0, 0, 0).unwrap();
    /// assert_eq!(ndt, NaiveDateTime::from(nd));
    fn from(date: NaiveDate) -> Self {
        date.and_hms(0, 0, 0).unwrap()
    }
}

impl Datelike for NaiveDateTime {
    /// Returns the year number in the [calendar date](./struct.NaiveDate.html#calendar-date).
    ///
    /// See also the [`NaiveDate::year`](./struct.NaiveDate.html#method.year) method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Datelike, NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 25).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(dt.year(), 2015);
    /// ```
    #[inline]
    fn year(&self) -> i32 {
        self.date.year()
    }

    /// Returns the month number starting from 1.
    ///
    /// The return value ranges from 1 to 12.
    ///
    /// See also the [`NaiveDate::month`](./struct.NaiveDate.html#method.month) method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Datelike, NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 25).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(dt.month(), 9);
    /// ```
    #[inline]
    fn month(&self) -> u32 {
        self.date.month()
    }

    /// Returns the month number starting from 0.
    ///
    /// The return value ranges from 0 to 11.
    ///
    /// See also the [`NaiveDate::month0`] method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Datelike, NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 25).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(dt.month0(), 8);
    /// ```
    #[inline]
    fn month0(&self) -> u32 {
        self.date.month0()
    }

    /// Returns the day of month starting from 1.
    ///
    /// The return value ranges from 1 to 31. (The last day of month differs by months.)
    ///
    /// See also the [`NaiveDate::day`](./struct.NaiveDate.html#method.day) method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Datelike, NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 25).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(dt.day(), 25);
    /// ```
    #[inline]
    fn day(&self) -> u32 {
        self.date.day()
    }

    /// Returns the day of month starting from 0.
    ///
    /// The return value ranges from 0 to 30. (The last day of month differs by months.)
    ///
    /// See also the [`NaiveDate::day0`] method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Datelike, NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 25).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(dt.day0(), 24);
    /// ```
    #[inline]
    fn day0(&self) -> u32 {
        self.date.day0()
    }

    /// Returns the day of year starting from 1.
    ///
    /// The return value ranges from 1 to 366. (The last day of year differs by years.)
    ///
    /// See also the [`NaiveDate::ordinal`](./struct.NaiveDate.html#method.ordinal) method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Datelike, NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 25).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(dt.ordinal(), 268);
    /// ```
    #[inline]
    fn ordinal(&self) -> u32 {
        self.date.ordinal()
    }

    /// Returns the day of year starting from 0.
    ///
    /// The return value ranges from 0 to 365. (The last day of year differs by years.)
    ///
    /// See also the [`NaiveDate::ordinal0`] method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Datelike, NaiveDate, NaiveDateTime};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 25).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(dt.ordinal0(), 267);
    /// ```
    #[inline]
    fn ordinal0(&self) -> u32 {
        self.date.ordinal0()
    }

    /// Returns the day of week.
    ///
    /// See also the [`NaiveDate::weekday`](./struct.NaiveDate.html#method.weekday) method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{Datelike, NaiveDate, NaiveDateTime, Weekday};
    ///
    /// let dt: NaiveDateTime = NaiveDate::from_ymd(2015, 9, 25).unwrap().and_hms(12, 34, 56).unwrap();
    /// assert_eq!(dt.weekday(), Weekday::Fri);
    /// ```
    #[inline]
    fn weekday(&self) -> Weekday {
        self.date.weekday()
    }

    #[inline]
    fn iso_week(&self) -> IsoWeek {
        self.date.iso_week()
    }
}

impl Timelike for NaiveDateTime {
    /// Returns the hour number from 0 to 23.
    ///
    /// See also the [`NaiveTime::hour`] method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, NaiveDateTime, Timelike};
    ///
    /// let dt: NaiveDateTime =
    ///     NaiveDate::from_ymd(2015, 9, 8).unwrap().and_hms_milli(12, 34, 56, 789).unwrap();
    /// assert_eq!(dt.hour(), 12);
    /// ```
    #[inline]
    fn hour(&self) -> u32 {
        self.time.hour()
    }

    /// Returns the minute number from 0 to 59.
    ///
    /// See also the [`NaiveTime::minute`] method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, NaiveDateTime, Timelike};
    ///
    /// let dt: NaiveDateTime =
    ///     NaiveDate::from_ymd(2015, 9, 8).unwrap().and_hms_milli(12, 34, 56, 789).unwrap();
    /// assert_eq!(dt.minute(), 34);
    /// ```
    #[inline]
    fn minute(&self) -> u32 {
        self.time.minute()
    }

    /// Returns the second number from 0 to 59.
    ///
    /// See also the [`NaiveTime::second`] method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, NaiveDateTime, Timelike};
    ///
    /// let dt: NaiveDateTime =
    ///     NaiveDate::from_ymd(2015, 9, 8).unwrap().and_hms_milli(12, 34, 56, 789).unwrap();
    /// assert_eq!(dt.second(), 56);
    /// ```
    #[inline]
    fn second(&self) -> u32 {
        self.time.second()
    }

    /// Returns the number of nanoseconds since the whole non-leap second.
    /// The range from 1,000,000,000 to 1,999,999,999 represents
    /// the [leap second](./struct.NaiveTime.html#leap-second-handling).
    ///
    /// See also the [`NaiveTime#method.nanosecond`] method.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::{NaiveDate, NaiveDateTime, Timelike};
    ///
    /// let dt: NaiveDateTime =
    ///     NaiveDate::from_ymd(2015, 9, 8).unwrap().and_hms_milli(12, 34, 56, 789).unwrap();
    /// assert_eq!(dt.nanosecond(), 789_000_000);
    /// ```
    #[inline]
    fn nanosecond(&self) -> u32 {
        self.time.nanosecond()
    }
}

/// Add `TimeDelta` to `NaiveDateTime`.
///
/// As a part of Chrono's [leap second handling], the addition assumes that **there is no leap
/// second ever**, except when the `NaiveDateTime` itself represents a leap  second in which case
/// the assumption becomes that **there is exactly a single leap second ever**.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using [`NaiveDateTime::checked_add_signed`] to get an `Option` instead.
///
/// # Example
///
/// ```
/// use chrono::{NaiveDate, TimeDelta};
///
/// let from_ymd = |y, m, d| NaiveDate::from_ymd(y, m, d).unwrap();
///
/// let d = from_ymd(2016, 7, 8);
/// let hms = |h, m, s| d.and_hms(h, m, s).unwrap();
/// assert_eq!(hms(3, 5, 7) + TimeDelta::zero(), hms(3, 5, 7));
/// assert_eq!(hms(3, 5, 7) + TimeDelta::seconds(1), hms(3, 5, 8));
/// assert_eq!(hms(3, 5, 7) + TimeDelta::seconds(-1), hms(3, 5, 6));
/// assert_eq!(hms(3, 5, 7) + TimeDelta::seconds(3600 + 60), hms(4, 6, 7));
/// assert_eq!(
///     hms(3, 5, 7) + TimeDelta::seconds(86_400),
///     from_ymd(2016, 7, 9).and_hms(3, 5, 7).unwrap()
/// );
/// assert_eq!(hms(3, 5, 7) + TimeDelta::days(365), from_ymd(2017, 7, 8).and_hms(3, 5, 7).unwrap());
///
/// let hmsm = |h, m, s, milli| d.and_hms_milli(h, m, s, milli).unwrap();
/// assert_eq!(hmsm(3, 5, 7, 980) + TimeDelta::milliseconds(450).unwrap(), hmsm(3, 5, 8, 430));
/// ```
///
/// Leap seconds are handled,
/// but the addition assumes that it is the only leap second happened.
///
/// ```
/// # use chrono::{TimeDelta, NaiveDate};
/// # let from_ymd = |y, m, d| NaiveDate::from_ymd(y, m, d).unwrap();
/// # let hmsm = |h, m, s, milli| from_ymd(2016, 7, 8).and_hms_milli(h, m, s, milli).unwrap();
/// let leap = hmsm(3, 5, 59, 1_300);
/// assert_eq!(leap + TimeDelta::zero(), hmsm(3, 5, 59, 1_300));
/// assert_eq!(leap + TimeDelta::milliseconds(-500).unwrap(), hmsm(3, 5, 59, 800));
/// assert_eq!(leap + TimeDelta::milliseconds(500).unwrap(), hmsm(3, 5, 59, 1_800));
/// assert_eq!(leap + TimeDelta::milliseconds(800).unwrap(), hmsm(3, 6, 0, 100));
/// assert_eq!(leap + TimeDelta::seconds(10), hmsm(3, 6, 9, 300));
/// assert_eq!(leap + TimeDelta::seconds(-10), hmsm(3, 5, 50, 300));
/// assert_eq!(leap + TimeDelta::days(1),
///            from_ymd(2016, 7, 9).and_hms_milli(3, 5, 59, 300).unwrap());
/// ```
///
/// [leap second handling]: crate::NaiveTime#leap-second-handling
impl Add<TimeDelta> for NaiveDateTime {
    type Output = NaiveDateTime;

    #[inline]
    fn add(self, rhs: TimeDelta) -> NaiveDateTime {
        self.checked_add_signed(rhs).expect("`NaiveDateTime + TimeDelta` overflowed")
    }
}

/// Add `std::time::Duration` to `NaiveDateTime`.
///
/// As a part of Chrono's [leap second handling], the addition assumes that **there is no leap
/// second ever**, except when the `NaiveDateTime` itself represents a leap  second in which case
/// the assumption becomes that **there is exactly a single leap second ever**.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using [`NaiveDateTime::checked_add_signed`] to get an `Option` instead.
impl Add<Duration> for NaiveDateTime {
    type Output = NaiveDateTime;

    #[inline]
    fn add(self, rhs: Duration) -> NaiveDateTime {
        let rhs = TimeDelta::from_std(rhs)
            .expect("overflow converting from core::time::Duration to TimeDelta");
        self.checked_add_signed(rhs).expect("`NaiveDateTime + TimeDelta` overflowed")
    }
}

/// Add-assign `TimeDelta` to `NaiveDateTime`.
///
/// As a part of Chrono's [leap second handling], the addition assumes that **there is no leap
/// second ever**, except when the `NaiveDateTime` itself represents a leap  second in which case
/// the assumption becomes that **there is exactly a single leap second ever**.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using [`NaiveDateTime::checked_add_signed`] to get an `Option` instead.
impl AddAssign<TimeDelta> for NaiveDateTime {
    #[inline]
    fn add_assign(&mut self, rhs: TimeDelta) {
        *self = self.add(rhs);
    }
}

/// Add-assign `std::time::Duration` to `NaiveDateTime`.
///
/// As a part of Chrono's [leap second handling], the addition assumes that **there is no leap
/// second ever**, except when the `NaiveDateTime` itself represents a leap  second in which case
/// the assumption becomes that **there is exactly a single leap second ever**.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using [`NaiveDateTime::checked_add_signed`] to get an `Option` instead.
impl AddAssign<Duration> for NaiveDateTime {
    #[inline]
    fn add_assign(&mut self, rhs: Duration) {
        *self = self.add(rhs);
    }
}

/// Add `FixedOffset` to `NaiveDateTime`.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using `checked_add_offset` to get an `Option` instead.
impl Add<FixedOffset> for NaiveDateTime {
    type Output = NaiveDateTime;

    #[inline]
    fn add(self, rhs: FixedOffset) -> NaiveDateTime {
        self.checked_add_offset(rhs).expect("`NaiveDateTime + FixedOffset` out of range")
    }
}

/// Add `Months` to `NaiveDateTime`.
///
/// The result will be clamped to valid days in the resulting month, see `checked_add_months` for
/// details.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using `checked_add_months` to get an `Option` instead.
///
/// # Example
///
/// ```
/// use chrono::{Months, NaiveDate};
///
/// assert_eq!(
///     NaiveDate::from_ymd(2014, 1, 1).unwrap().and_hms(1, 0, 0).unwrap() + Months::new(1),
///     NaiveDate::from_ymd(2014, 2, 1).unwrap().and_hms(1, 0, 0).unwrap()
/// );
/// assert_eq!(
///     NaiveDate::from_ymd(2014, 1, 1).unwrap().and_hms(0, 2, 0).unwrap() + Months::new(11),
///     NaiveDate::from_ymd(2014, 12, 1).unwrap().and_hms(0, 2, 0).unwrap()
/// );
/// assert_eq!(
///     NaiveDate::from_ymd(2014, 1, 1).unwrap().and_hms(0, 0, 3).unwrap() + Months::new(12),
///     NaiveDate::from_ymd(2015, 1, 1).unwrap().and_hms(0, 0, 3).unwrap()
/// );
/// assert_eq!(
///     NaiveDate::from_ymd(2014, 1, 1).unwrap().and_hms(0, 0, 4).unwrap() + Months::new(13),
///     NaiveDate::from_ymd(2015, 2, 1).unwrap().and_hms(0, 0, 4).unwrap()
/// );
/// assert_eq!(
///     NaiveDate::from_ymd(2014, 1, 31).unwrap().and_hms(0, 5, 0).unwrap() + Months::new(1),
///     NaiveDate::from_ymd(2014, 2, 28).unwrap().and_hms(0, 5, 0).unwrap()
/// );
/// assert_eq!(
///     NaiveDate::from_ymd(2020, 1, 31).unwrap().and_hms(6, 0, 0).unwrap() + Months::new(1),
///     NaiveDate::from_ymd(2020, 2, 29).unwrap().and_hms(6, 0, 0).unwrap()
/// );
/// ```
impl Add<Months> for NaiveDateTime {
    type Output = NaiveDateTime;

    fn add(self, rhs: Months) -> Self::Output {
        self.checked_add_months(rhs).expect("`NaiveDateTime + Months` out of range")
    }
}

/// Subtract `TimeDelta` from `NaiveDateTime`.
///
/// This is the same as the addition with a negated `TimeDelta`.
///
/// As a part of Chrono's [leap second handling] the subtraction assumes that **there is no leap
/// second ever**, except when the `NaiveDateTime` itself represents a leap second in which case
/// the assumption becomes that **there is exactly a single leap second ever**.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using [`NaiveDateTime::checked_sub_signed`] to get an `Option` instead.
///
/// # Example
///
/// ```
/// use chrono::{NaiveDate, TimeDelta};
///
/// let from_ymd = |y, m, d| NaiveDate::from_ymd(y, m, d).unwrap();
///
/// let d = from_ymd(2016, 7, 8);
/// let hms = |h, m, s| d.and_hms(h, m, s).unwrap();
/// assert_eq!(hms(3, 5, 7) - TimeDelta::zero(), hms(3, 5, 7));
/// assert_eq!(hms(3, 5, 7) - TimeDelta::seconds(1), hms(3, 5, 6));
/// assert_eq!(hms(3, 5, 7) - TimeDelta::seconds(-1), hms(3, 5, 8));
/// assert_eq!(hms(3, 5, 7) - TimeDelta::seconds(3600 + 60), hms(2, 4, 7));
/// assert_eq!(
///     hms(3, 5, 7) - TimeDelta::seconds(86_400),
///     from_ymd(2016, 7, 7).and_hms(3, 5, 7).unwrap()
/// );
/// assert_eq!(hms(3, 5, 7) - TimeDelta::days(365), from_ymd(2015, 7, 9).and_hms(3, 5, 7).unwrap());
///
/// let hmsm = |h, m, s, milli| d.and_hms_milli(h, m, s, milli).unwrap();
/// assert_eq!(hmsm(3, 5, 7, 450) - TimeDelta::milliseconds(670).unwrap(), hmsm(3, 5, 6, 780));
/// ```
///
/// Leap seconds are handled,
/// but the subtraction assumes that it is the only leap second happened.
///
/// ```
/// # use chrono::{TimeDelta, NaiveDate};
/// # let from_ymd = |y, m, d| NaiveDate::from_ymd(y, m, d).unwrap();
/// # let hmsm = |h, m, s, milli| from_ymd(2016, 7, 8).and_hms_milli(h, m, s, milli).unwrap();
/// let leap = hmsm(3, 5, 59, 1_300);
/// assert_eq!(leap - TimeDelta::zero(), hmsm(3, 5, 59, 1_300));
/// assert_eq!(leap - TimeDelta::milliseconds(200).unwrap(), hmsm(3, 5, 59, 1_100));
/// assert_eq!(leap - TimeDelta::milliseconds(500).unwrap(), hmsm(3, 5, 59, 800));
/// assert_eq!(leap - TimeDelta::seconds(60), hmsm(3, 5, 0, 300));
/// assert_eq!(leap - TimeDelta::days(1),
///            from_ymd(2016, 7, 7).and_hms_milli(3, 6, 0, 300).unwrap());
/// ```
///
/// [leap second handling]: crate::NaiveTime#leap-second-handling
impl Sub<TimeDelta> for NaiveDateTime {
    type Output = NaiveDateTime;

    #[inline]
    fn sub(self, rhs: TimeDelta) -> NaiveDateTime {
        self.checked_sub_signed(rhs).expect("`NaiveDateTime - TimeDelta` overflowed")
    }
}

/// Subtract `std::time::Duration` from `NaiveDateTime`.
///
/// As a part of Chrono's [leap second handling] the subtraction assumes that **there is no leap
/// second ever**, except when the `NaiveDateTime` itself represents a leap second in which case
/// the assumption becomes that **there is exactly a single leap second ever**.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using [`NaiveDateTime::checked_sub_signed`] to get an `Option` instead.
impl Sub<Duration> for NaiveDateTime {
    type Output = NaiveDateTime;

    #[inline]
    fn sub(self, rhs: Duration) -> NaiveDateTime {
        let rhs = TimeDelta::from_std(rhs)
            .expect("overflow converting from core::time::Duration to TimeDelta");
        self.checked_sub_signed(rhs).expect("`NaiveDateTime - TimeDelta` overflowed")
    }
}

/// Subtract-assign `TimeDelta` from `NaiveDateTime`.
///
/// This is the same as the addition with a negated `TimeDelta`.
///
/// As a part of Chrono's [leap second handling], the addition assumes that **there is no leap
/// second ever**, except when the `NaiveDateTime` itself represents a leap  second in which case
/// the assumption becomes that **there is exactly a single leap second ever**.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using [`NaiveDateTime::checked_sub_signed`] to get an `Option` instead.
impl SubAssign<TimeDelta> for NaiveDateTime {
    #[inline]
    fn sub_assign(&mut self, rhs: TimeDelta) {
        *self = self.sub(rhs);
    }
}

/// Subtract-assign `std::time::Duration` from `NaiveDateTime`.
///
/// As a part of Chrono's [leap second handling], the addition assumes that **there is no leap
/// second ever**, except when the `NaiveDateTime` itself represents a leap  second in which case
/// the assumption becomes that **there is exactly a single leap second ever**.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using [`NaiveDateTime::checked_sub_signed`] to get an `Option` instead.
impl SubAssign<Duration> for NaiveDateTime {
    #[inline]
    fn sub_assign(&mut self, rhs: Duration) {
        *self = self.sub(rhs);
    }
}

/// Subtract `FixedOffset` from `NaiveDateTime`.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using `checked_sub_offset` to get an `Option` instead.
impl Sub<FixedOffset> for NaiveDateTime {
    type Output = NaiveDateTime;

    #[inline]
    fn sub(self, rhs: FixedOffset) -> NaiveDateTime {
        self.checked_sub_offset(rhs).expect("`NaiveDateTime - FixedOffset` out of range")
    }
}

/// Subtract `Months` from `NaiveDateTime`.
///
/// The result will be clamped to valid days in the resulting month, see
/// [`NaiveDateTime::checked_sub_months`] for details.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using [`NaiveDateTime::checked_sub_months`] to get an `Option` instead.
///
/// # Example
///
/// ```
/// use chrono::{Months, NaiveDate};
///
/// assert_eq!(
///     NaiveDate::from_ymd(2014, 01, 01).unwrap().and_hms(01, 00, 00).unwrap() - Months::new(11),
///     NaiveDate::from_ymd(2013, 02, 01).unwrap().and_hms(01, 00, 00).unwrap()
/// );
/// assert_eq!(
///     NaiveDate::from_ymd(2014, 01, 01).unwrap().and_hms(00, 02, 00).unwrap() - Months::new(12),
///     NaiveDate::from_ymd(2013, 01, 01).unwrap().and_hms(00, 02, 00).unwrap()
/// );
/// assert_eq!(
///     NaiveDate::from_ymd(2014, 01, 01).unwrap().and_hms(00, 00, 03).unwrap() - Months::new(13),
///     NaiveDate::from_ymd(2012, 12, 01).unwrap().and_hms(00, 00, 03).unwrap()
/// );
/// ```
impl Sub<Months> for NaiveDateTime {
    type Output = NaiveDateTime;

    fn sub(self, rhs: Months) -> Self::Output {
        self.checked_sub_months(rhs).expect("`NaiveDateTime - Months` out of range")
    }
}

/// Subtracts another `NaiveDateTime` from the current date and time.
/// This does not overflow or underflow at all.
///
/// As a part of Chrono's [leap second handling](./struct.NaiveTime.html#leap-second-handling),
/// the subtraction assumes that **there is no leap second ever**,
/// except when any of the `NaiveDateTime`s themselves represents a leap second
/// in which case the assumption becomes that
/// **there are exactly one (or two) leap second(s) ever**.
///
/// The implementation is a wrapper around [`NaiveDateTime::signed_duration_since`].
///
/// # Example
///
/// ```
/// use chrono::{NaiveDate, TimeDelta};
///
/// let from_ymd = |y, m, d| NaiveDate::from_ymd(y, m, d).unwrap();
///
/// let d = from_ymd(2016, 7, 8);
/// assert_eq!(
///     d.and_hms(3, 5, 7).unwrap() - d.and_hms(2, 4, 6).unwrap(),
///     TimeDelta::seconds(3600 + 60 + 1)
/// );
///
/// // July 8 is 190th day in the year 2016
/// let d0 = from_ymd(2016, 1, 1);
/// assert_eq!(
///     d.and_hms_milli(0, 7, 6, 500).unwrap() - d0.and_hms(0, 0, 0).unwrap(),
///     TimeDelta::seconds(189 * 86_400 + 7 * 60 + 6) + TimeDelta::milliseconds(500).unwrap()
/// );
/// ```
///
/// Leap seconds are handled, but the subtraction assumes that no other leap
/// seconds happened.
///
/// ```
/// # use chrono::{TimeDelta, NaiveDate};
/// # let from_ymd = |y, m, d| NaiveDate::from_ymd(y, m, d).unwrap();
/// let leap = from_ymd(2015, 6, 30).and_hms_milli(23, 59, 59, 1_500).unwrap();
/// assert_eq!(
///     leap - from_ymd(2015, 6, 30).and_hms(23, 0, 0).unwrap(),
///     TimeDelta::seconds(3600) + TimeDelta::milliseconds(500).unwrap()
/// );
/// assert_eq!(
///     from_ymd(2015, 7, 1).and_hms(1, 0, 0).unwrap() - leap,
///     TimeDelta::seconds(3600) - TimeDelta::milliseconds(500).unwrap()
/// );
/// ```
impl Sub<NaiveDateTime> for NaiveDateTime {
    type Output = TimeDelta;

    #[inline]
    fn sub(self, rhs: NaiveDateTime) -> TimeDelta {
        self.signed_duration_since(rhs)
    }
}

/// Add `Days` to `NaiveDateTime`.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using `checked_add_days` to get an `Option` instead.
impl Add<Days> for NaiveDateTime {
    type Output = NaiveDateTime;

    fn add(self, days: Days) -> Self::Output {
        self.checked_add_days(days).expect("`NaiveDateTime + Days` out of range")
    }
}

/// Subtract `Days` from `NaiveDateTime`.
///
/// # Panics
///
/// Panics if the resulting date would be out of range.
/// Consider using `checked_sub_days` to get an `Option` instead.
impl Sub<Days> for NaiveDateTime {
    type Output = NaiveDateTime;

    fn sub(self, days: Days) -> Self::Output {
        self.checked_sub_days(days).expect("`NaiveDateTime - Days` out of range")
    }
}

/// The `Debug` output of the naive date and time `dt` is the same as
/// [`dt.format("%Y-%m-%dT%H:%M:%S%.f")`](crate::format::strftime).
///
/// The string printed can be readily parsed via the `parse` method on `str`.
///
/// It should be noted that, for leap seconds not on the minute boundary,
/// it may print a representation not distinguishable from non-leap seconds.
/// This doesn't matter in practice, since such leap seconds never happened.
/// (By the time of the first leap second on 1972-06-30,
/// every time zone offset around the world has standardized to the 5-minute alignment.)
///
/// # Example
///
/// ```
/// use chrono::NaiveDate;
///
/// let dt = NaiveDate::from_ymd(2016, 11, 15).unwrap().and_hms(7, 39, 24).unwrap();
/// assert_eq!(format!("{:?}", dt), "2016-11-15T07:39:24");
/// ```
///
/// Leap seconds may also be used.
///
/// ```
/// # use chrono::NaiveDate;
/// let dt = NaiveDate::from_ymd(2015, 6, 30).unwrap().and_hms_milli(23, 59, 59, 1_500).unwrap();
/// assert_eq!(format!("{:?}", dt), "2015-06-30T23:59:60.500");
/// ```
impl fmt::Debug for NaiveDateTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.date.fmt(f)?;
        f.write_char('T')?;
        self.time.fmt(f)
    }
}

/// The `Display` output of the naive date and time `dt` is the same as
/// [`dt.format("%Y-%m-%d %H:%M:%S%.f")`](crate::format::strftime).
///
/// It should be noted that, for leap seconds not on the minute boundary,
/// it may print a representation not distinguishable from non-leap seconds.
/// This doesn't matter in practice, since such leap seconds never happened.
/// (By the time of the first leap second on 1972-06-30,
/// every time zone offset around the world has standardized to the 5-minute alignment.)
///
/// # Example
///
/// ```
/// use chrono::NaiveDate;
///
/// let dt = NaiveDate::from_ymd(2016, 11, 15).unwrap().and_hms(7, 39, 24).unwrap();
/// assert_eq!(format!("{}", dt), "2016-11-15 07:39:24");
/// ```
///
/// Leap seconds may also be used.
///
/// ```
/// # use chrono::NaiveDate;
/// let dt = NaiveDate::from_ymd(2015, 6, 30).unwrap().and_hms_milli(23, 59, 59, 1_500).unwrap();
/// assert_eq!(format!("{}", dt), "2015-06-30 23:59:60.500");
/// ```
impl fmt::Display for NaiveDateTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.date.fmt(f)?;
        f.write_char(' ')?;
        self.time.fmt(f)
    }
}

/// Parsing a `str` into a `NaiveDateTime` uses the same format,
/// [`%Y-%m-%dT%H:%M:%S%.f`](crate::format::strftime), as in `Debug`.
///
/// # Example
///
/// ```
/// use chrono::{NaiveDateTime, NaiveDate};
///
/// let dt = NaiveDate::from_ymd(2015, 9, 18).unwrap().and_hms(23, 56, 4).unwrap();
/// assert_eq!("2015-09-18T23:56:04".parse::<NaiveDateTime>(), Ok(dt));
///
/// let dt = NaiveDate::from_ymd(12345, 6, 7).unwrap().and_hms_milli(7, 59, 59, 1_500).unwrap(); // leap second
/// assert_eq!("+12345-6-7T7:59:60.5".parse::<NaiveDateTime>(), Ok(dt));
///
/// assert!("foo".parse::<NaiveDateTime>().is_err());
/// ```
impl str::FromStr for NaiveDateTime {
    type Err = ParseError;

    fn from_str(s: &str) -> ParseResult<NaiveDateTime> {
        const ITEMS: &[Item<'static>] = &[
            Item::Numeric(Numeric::Year, Pad::Zero),
            Item::Space(""),
            Item::Literal("-"),
            Item::Numeric(Numeric::Month, Pad::Zero),
            Item::Space(""),
            Item::Literal("-"),
            Item::Numeric(Numeric::Day, Pad::Zero),
            Item::Space(""),
            Item::Literal("T"), // XXX shouldn't this be case-insensitive?
            Item::Numeric(Numeric::Hour, Pad::Zero),
            Item::Space(""),
            Item::Literal(":"),
            Item::Numeric(Numeric::Minute, Pad::Zero),
            Item::Space(""),
            Item::Literal(":"),
            Item::Numeric(Numeric::Second, Pad::Zero),
            Item::Fixed(Fixed::Nanosecond),
            Item::Space(""),
        ];

        let mut parsed = Parsed::default();
        parse(&mut parsed, s, ITEMS.iter())?;
        parsed.to_naive_datetime_with_offset(0)
    }
}

/// The default value for a NaiveDateTime is one with epoch 0
/// that is, 1st of January 1970 at 00:00:00.
///
/// # Example
///
/// ```rust
/// use chrono::NaiveDateTime;
///
/// assert_eq!(NaiveDateTime::default(), NaiveDateTime::UNIX_EPOCH);
/// ```
impl Default for NaiveDateTime {
    fn default() -> Self {
        Self::UNIX_EPOCH
    }
}

#[cfg(all(test, feature = "serde"))]
fn test_encodable_json<F, E>(to_string: F)
where
    F: Fn(&NaiveDateTime) -> Result<String, E>,
    E: ::std::fmt::Debug,
{
    assert_eq!(
        to_string(&NaiveDate::from_ymd(2016, 7, 8).unwrap().and_hms_milli(9, 10, 48, 90).unwrap())
            .ok(),
        Some(r#""2016-07-08T09:10:48.090""#.into())
    );
    assert_eq!(
        to_string(&NaiveDate::from_ymd(2014, 7, 24).unwrap().and_hms(12, 34, 6).unwrap()).ok(),
        Some(r#""2014-07-24T12:34:06""#.into())
    );
    assert_eq!(
        to_string(&NaiveDate::from_ymd(0, 1, 1).unwrap().and_hms_milli(0, 0, 59, 1_000).unwrap())
            .ok(),
        Some(r#""0000-01-01T00:00:60""#.into())
    );
    assert_eq!(
        to_string(&NaiveDate::from_ymd(-1, 12, 31).unwrap().and_hms_nano(23, 59, 59, 7).unwrap())
            .ok(),
        Some(r#""-0001-12-31T23:59:59.000000007""#.into())
    );
    assert_eq!(
        to_string(&NaiveDate::MIN.and_hms(0, 0, 0).unwrap()).ok(),
        Some(r#""-262143-01-01T00:00:00""#.into())
    );
    assert_eq!(
        to_string(&NaiveDate::MAX.and_hms_nano(23, 59, 59, 1_999_999_999).unwrap()).ok(),
        Some(r#""+262142-12-31T23:59:60.999999999""#.into())
    );
}

#[cfg(all(test, feature = "serde"))]
fn test_decodable_json<F, E>(from_str: F)
where
    F: Fn(&str) -> Result<NaiveDateTime, E>,
    E: ::std::fmt::Debug,
{
    assert_eq!(
        from_str(r#""2016-07-08T09:10:48.090""#).ok(),
        Some(NaiveDate::from_ymd(2016, 7, 8).unwrap().and_hms_milli(9, 10, 48, 90).unwrap())
    );
    assert_eq!(
        from_str(r#""2016-7-8T9:10:48.09""#).ok(),
        Some(NaiveDate::from_ymd(2016, 7, 8).unwrap().and_hms_milli(9, 10, 48, 90).unwrap())
    );
    assert_eq!(
        from_str(r#""2014-07-24T12:34:06""#).ok(),
        Some(NaiveDate::from_ymd(2014, 7, 24).unwrap().and_hms(12, 34, 6).unwrap())
    );
    assert_eq!(
        from_str(r#""0000-01-01T00:00:60""#).ok(),
        Some(NaiveDate::from_ymd(0, 1, 1).unwrap().and_hms_milli(0, 0, 59, 1_000).unwrap())
    );
    assert_eq!(
        from_str(r#""0-1-1T0:0:60""#).ok(),
        Some(NaiveDate::from_ymd(0, 1, 1).unwrap().and_hms_milli(0, 0, 59, 1_000).unwrap())
    );
    assert_eq!(
        from_str(r#""-0001-12-31T23:59:59.000000007""#).ok(),
        Some(NaiveDate::from_ymd(-1, 12, 31).unwrap().and_hms_nano(23, 59, 59, 7).unwrap())
    );
    assert_eq!(
        from_str(r#""-262143-01-01T00:00:00""#).ok(),
        Some(NaiveDate::MIN.and_hms(0, 0, 0).unwrap())
    );
    assert_eq!(
        from_str(r#""+262142-12-31T23:59:60.999999999""#).ok(),
        Some(NaiveDate::MAX.and_hms_nano(23, 59, 59, 1_999_999_999).unwrap())
    );
    assert_eq!(
        from_str(r#""+262142-12-31T23:59:60.9999999999997""#).ok(), // excess digits are ignored
        Some(NaiveDate::MAX.and_hms_nano(23, 59, 59, 1_999_999_999).unwrap())
    );

    // bad formats
    assert!(from_str(r#""""#).is_err());
    assert!(from_str(r#""2016-07-08""#).is_err());
    assert!(from_str(r#""09:10:48.090""#).is_err());
    assert!(from_str(r#""20160708T091048.090""#).is_err());
    assert!(from_str(r#""2000-00-00T00:00:00""#).is_err());
    assert!(from_str(r#""2000-02-30T00:00:00""#).is_err());
    assert!(from_str(r#""2001-02-29T00:00:00""#).is_err());
    assert!(from_str(r#""2002-02-28T24:00:00""#).is_err());
    assert!(from_str(r#""2002-02-28T23:60:00""#).is_err());
    assert!(from_str(r#""2002-02-28T23:59:61""#).is_err());
    assert!(from_str(r#""2016-07-08T09:10:48,090""#).is_err());
    assert!(from_str(r#""2016-07-08 09:10:48.090""#).is_err());
    assert!(from_str(r#""2016-007-08T09:10:48.090""#).is_err());
    assert!(from_str(r#""yyyy-mm-ddThh:mm:ss.fffffffff""#).is_err());
    assert!(from_str(r#"20160708000000"#).is_err());
    assert!(from_str(r#"{}"#).is_err());
    // pre-0.3.0 rustc-serialize format is now invalid
    assert!(from_str(r#"{"date":{"ymdf":20},"time":{"secs":0,"frac":0}}"#).is_err());
    assert!(from_str(r#"null"#).is_err());
}

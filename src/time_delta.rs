// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Temporal quantification

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};
use core::time::Duration;
use core::{fmt, i64};

use crate::{expect, ok, try_ok_or, try_opt, Error};

#[cfg(any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"))]
use rkyv::{Archive, Deserialize, Serialize};

/// The number of nanoseconds in a microsecond.
const NANOS_PER_MICRO: i32 = 1000;
/// The number of nanoseconds in a millisecond.
const NANOS_PER_MILLI: i32 = 1_000_000;
/// The number of nanoseconds in seconds.
const NANOS_PER_SEC: i32 = 1_000_000_000;
/// The number of microseconds per second.
const MICROS_PER_SEC: i64 = 1_000_000;
/// The number of milliseconds per second.
const MILLIS_PER_SEC: i64 = 1000;
/// The number of seconds in a minute.
const SECS_PER_MINUTE: i64 = 60;
/// The number of seconds in an hour.
const SECS_PER_HOUR: i64 = 3600;
/// The number of (non-leap) seconds in days.
const SECS_PER_DAY: i64 = 86_400;
/// The number of (non-leap) seconds in a week.
const SECS_PER_WEEK: i64 = 604_800;

/// Time duration with nanosecond precision.
///
/// This also allows for negative durations; see individual methods for details.
///
/// A `TimeDelta` is represented internally as a complement of seconds and
/// nanoseconds. The range is restricted to that of `i64` milliseconds, with the
/// minimum value notably being set to `-i64::MAX` rather than allowing the full
/// range of `i64::MIN`. This is to allow easy flipping of sign, so that for
/// instance `abs()` can be called without any checks.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[cfg_attr(
    any(feature = "rkyv", feature = "rkyv-16", feature = "rkyv-32", feature = "rkyv-64"),
    derive(Archive, Deserialize, Serialize),
    archive(compare(PartialEq, PartialOrd)),
    archive_attr(derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash))
)]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct TimeDelta {
    secs: i64,
    nanos: i32, // Always 0 <= nanos < NANOS_PER_SEC
}

/// The minimum possible `TimeDelta`: `-i64::MAX` milliseconds.
pub(crate) const MIN: TimeDelta = TimeDelta {
    secs: -i64::MAX / MILLIS_PER_SEC - 1,
    nanos: NANOS_PER_SEC + (-i64::MAX % MILLIS_PER_SEC) as i32 * NANOS_PER_MILLI,
};

/// The maximum possible `TimeDelta`: `i64::MAX` milliseconds.
pub(crate) const MAX: TimeDelta = TimeDelta {
    secs: i64::MAX / MILLIS_PER_SEC,
    nanos: (i64::MAX % MILLIS_PER_SEC) as i32 * NANOS_PER_MILLI,
};

impl TimeDelta {
    /// Makes a new `TimeDelta` with given number of seconds and nanoseconds.
    ///
    /// # Errors
    ///
    /// This method returns:
    /// - [`Error::OutOfRange`] when the duration is out of bounds.
    /// - [`Error::InvalidArgument`] if `nanos` ≥ 1,000,000,000.
    pub const fn new(secs: i64, nanos: u32) -> Result<TimeDelta, Error> {
        if nanos >= 1_000_000_000 {
            return Err(Error::InvalidArgument);
        }
        if secs < MIN.secs
            || secs > MAX.secs
            || (secs == MAX.secs && nanos > MAX.nanos as u32)
            || (secs == MIN.secs && nanos < MIN.nanos as u32)
        {
            return Err(Error::OutOfRange);
        }
        Ok(TimeDelta { secs, nanos: nanos as i32 })
    }

    /// Makes a new `TimeDelta` with the given number of weeks.
    ///
    /// Equivalent to `TimeDelta::new(weeks as i64 * 7 * 24 * 60 * 60).unwrap()`.
    #[inline]
    pub const fn weeks(weeks: i32) -> TimeDelta {
        expect(ok!(TimeDelta::new(weeks as i64 * SECS_PER_WEEK, 0)), "always in range")
    }

    /// Makes a new `TimeDelta` with the given number of days.
    ///
    /// Equivalent to `TimeDelta::new(days as i64 * 24 * 60 * 60).unwrap()`.
    #[inline]
    pub const fn days(days: i32) -> TimeDelta {
        expect(ok!(TimeDelta::new(days as i64 * SECS_PER_DAY, 0)), "always in range")
    }

    /// Makes a new `TimeDelta` with the given number of hours.
    ///
    /// Equivalent to `TimeDelta::new(hours as i64 * 60 * 60, 0).unwrap()`.
    #[inline]
    pub const fn hours(hours: i32) -> TimeDelta {
        expect(ok!(TimeDelta::new(hours as i64 * SECS_PER_HOUR, 0)), "always in range")
    }

    /// Makes a new `TimeDelta` with the given number of minutes.
    ///
    /// Equivalent to `TimeDelta::new(minutes as i64 * 60, 0).unwrap()`.
    #[inline]
    pub const fn minutes(minutes: i32) -> TimeDelta {
        expect(ok!(TimeDelta::new(minutes as i64 * SECS_PER_MINUTE, 0)), "always in range")
    }

    /// Makes a new `TimeDelta` with the given number of seconds.
    ///
    /// Use [`TimeDelta::new()`] if you want to create a value `TimeDelta` with a range that is
    /// greater than that of an `i32`.
    #[inline]
    pub const fn seconds(seconds: i32) -> TimeDelta {
        expect(ok!(TimeDelta::new(seconds as i64, 0)), "always in range")
    }

    /// Makes a new `TimeDelta` with the given number of milliseconds.
    ///
    /// # Errors
    ///
    /// Returns `None` the `TimeDelta` would be out of bounds, i.e. when `milliseconds` is more
    /// than `i64::MAX` or less than `-i64::MAX`. Notably, this is not the same as `i64::MIN`.
    #[inline]
    pub const fn milliseconds(milliseconds: i64) -> Result<TimeDelta, Error> {
        // We don't need to compare against MAX, as this function accepts an
        // i64, and MAX is aligned to i64::MAX milliseconds.
        if milliseconds < -i64::MAX {
            return Err(Error::OutOfRange);
        }
        let (secs, millis) = div_mod_floor_64(milliseconds, MILLIS_PER_SEC);
        let d = TimeDelta { secs, nanos: millis as i32 * NANOS_PER_MILLI };
        Ok(d)
    }

    /// Makes a new `TimeDelta` with the given number of microseconds.
    ///
    /// The number of microseconds acceptable by this constructor is less than
    /// the total number that can actually be stored in a `TimeDelta`, so it is
    /// not possible to specify a value that would be out of bounds. This
    /// function is therefore infallible.
    #[inline]
    pub const fn microseconds(microseconds: i64) -> TimeDelta {
        let (secs, micros) = div_mod_floor_64(microseconds, MICROS_PER_SEC);
        let nanos = micros as i32 * NANOS_PER_MICRO;
        TimeDelta { secs, nanos }
    }

    /// Makes a new `TimeDelta` with the given number of nanoseconds.
    ///
    /// The number of nanoseconds acceptable by this constructor is less than
    /// the total number that can actually be stored in a `TimeDelta`, so it is
    /// not possible to specify a value that would be out of bounds. This
    /// function is therefore infallible.
    #[inline]
    pub const fn nanoseconds(nanos: i64) -> TimeDelta {
        let (secs, nanos) = div_mod_floor_64(nanos, NANOS_PER_SEC as i64);
        TimeDelta { secs, nanos: nanos as i32 }
    }

    /// Returns the total number of whole weeks in the `TimeDelta`.
    #[inline]
    pub const fn num_weeks(self) -> i64 {
        self.num_days() / 7
    }

    /// Returns the total number of whole days in the `TimeDelta`.
    pub const fn num_days(self) -> i64 {
        self.num_seconds() / SECS_PER_DAY
    }

    /// Returns the total number of whole hours in the `TimeDelta`.
    #[inline]
    pub const fn num_hours(self) -> i64 {
        self.num_seconds() / SECS_PER_HOUR
    }

    /// Returns the total number of whole minutes in the `TimeDelta`.
    #[inline]
    pub const fn num_minutes(self) -> i64 {
        self.num_seconds() / SECS_PER_MINUTE
    }

    /// Returns the total number of whole seconds in the `TimeDelta`.
    pub const fn num_seconds(self) -> i64 {
        // If secs is negative, nanos should be subtracted from the duration.
        if self.secs < 0 && self.nanos > 0 {
            self.secs + 1
        } else {
            self.secs
        }
    }

    /// Returns the number of nanoseconds such that
    /// `subsec_nanos() + num_seconds() * NANOS_PER_SEC` is the total number of
    /// nanoseconds in the `TimeDelta`.
    pub const fn subsec_nanos(self) -> i32 {
        if self.secs < 0 && self.nanos > 0 {
            self.nanos - NANOS_PER_SEC
        } else {
            self.nanos
        }
    }

    /// Returns the total number of whole milliseconds in the `TimeDelta`.
    pub const fn num_milliseconds(self) -> i64 {
        // A proper TimeDelta will not overflow, because MIN and MAX are defined such
        // that the range is within the bounds of an i64, from -i64::MAX through to
        // +i64::MAX inclusive. Notably, i64::MIN is excluded from this range.
        let secs_part = self.num_seconds() * MILLIS_PER_SEC;
        let nanos_part = self.subsec_nanos() / NANOS_PER_MILLI;
        secs_part + nanos_part as i64
    }

    /// Returns the total number of whole microseconds in the `TimeDelta`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::OutOfRange`]  on overflow (exceeding 2^63 microseconds in either
    /// direction).
    pub const fn num_microseconds(self) -> Result<i64, Error> {
        let secs_part =
            try_ok_or!(self.num_seconds().checked_mul(MICROS_PER_SEC), Error::OutOfRange);
        let nanos_part = self.subsec_nanos() / NANOS_PER_MICRO;
        Ok(try_ok_or!(secs_part.checked_add(nanos_part as i64), Error::OutOfRange))
    }

    /// Returns the total number of whole nanoseconds in the `TimeDelta`,
    /// or `None` on overflow (exceeding 2^63 nanoseconds in either direction).
    pub const fn num_nanoseconds(self) -> Option<i64> {
        let secs_part = try_opt!(self.num_seconds().checked_mul(NANOS_PER_SEC as i64));
        let nanos_part = self.subsec_nanos();
        secs_part.checked_add(nanos_part as i64)
    }

    /// Add two `TimeDelta`s, returning `None` if overflow occurred.
    #[must_use]
    pub const fn checked_add(self, rhs: TimeDelta) -> Option<TimeDelta> {
        // No overflow checks here because we stay comfortably within the range of an `i64`.
        // Range checks happen in `TimeDelta::new`.
        let mut secs = self.secs + rhs.secs;
        let mut nanos = self.nanos + rhs.nanos;
        if nanos >= NANOS_PER_SEC {
            nanos -= NANOS_PER_SEC;
            secs += 1;
        }
        ok!(TimeDelta::new(secs, nanos as u32))
    }

    /// Subtract two `TimeDelta`s, returning `None` if overflow occurred.
    #[must_use]
    pub const fn checked_sub(self, rhs: TimeDelta) -> Option<TimeDelta> {
        // No overflow checks here because we stay comfortably within the range of an `i64`.
        // Range checks happen in `TimeDelta::new`.
        let mut secs = self.secs - rhs.secs;
        let mut nanos = self.nanos - rhs.nanos;
        if nanos < 0 {
            nanos += NANOS_PER_SEC;
            secs -= 1;
        }
        ok!(TimeDelta::new(secs, nanos as u32))
    }

    /// Returns the `TimeDelta` as an absolute (non-negative) value.
    #[inline]
    pub const fn abs(self) -> TimeDelta {
        if self.secs < 0 && self.nanos != 0 {
            TimeDelta { secs: (self.secs + 1).abs(), nanos: NANOS_PER_SEC - self.nanos }
        } else {
            TimeDelta { secs: self.secs.abs(), nanos: self.nanos }
        }
    }

    /// The minimum possible `TimeDelta`: `-i64::MAX` milliseconds.
    #[inline]
    pub const fn min_value() -> TimeDelta {
        MIN
    }

    /// The maximum possible `TimeDelta`: `i64::MAX` milliseconds.
    #[inline]
    pub const fn max_value() -> TimeDelta {
        MAX
    }

    /// A `TimeDelta` where the stored seconds and nanoseconds are equal to zero.
    #[inline]
    pub const fn zero() -> TimeDelta {
        TimeDelta { secs: 0, nanos: 0 }
    }

    /// Returns `true` if the `TimeDelta` equals `TimeDelta::zero()`.
    #[inline]
    pub const fn is_zero(self) -> bool {
        self.secs == 0 && self.nanos == 0
    }

    /// Creates a `TimeDelta` object from `std::time::Duration`
    ///
    /// This function errors when original duration is larger than the maximum
    /// value supported for this type.
    pub const fn from_std(duration: Duration) -> Result<TimeDelta, OutOfRangeError> {
        // We need to check secs as u64 before coercing to i64
        if duration.as_secs() > MAX.secs as u64 {
            return Err(OutOfRangeError(()));
        }
        match TimeDelta::new(duration.as_secs() as i64, duration.subsec_nanos()) {
            Ok(d) => Ok(d),
            Err(_) => Err(OutOfRangeError(())),
        }
    }

    /// Creates a `std::time::Duration` object from a `TimeDelta`.
    ///
    /// This function errors when duration is less than zero. As standard
    /// library implementation is limited to non-negative values.
    pub const fn to_std(self) -> Result<Duration, OutOfRangeError> {
        if self.secs < 0 {
            return Err(OutOfRangeError(()));
        }
        Ok(Duration::new(self.secs as u64, self.nanos as u32))
    }

    /// This duplicates `Neg::neg` because trait methods can't be const yet.
    pub(crate) const fn neg(self) -> TimeDelta {
        let (secs_diff, nanos) = match self.nanos {
            0 => (0, 0),
            nanos => (1, NANOS_PER_SEC - nanos),
        };
        TimeDelta { secs: -self.secs - secs_diff, nanos }
    }
}

impl Neg for TimeDelta {
    type Output = TimeDelta;

    #[inline]
    fn neg(self) -> TimeDelta {
        let (secs_diff, nanos) = match self.nanos {
            0 => (0, 0),
            nanos => (1, NANOS_PER_SEC - nanos),
        };
        TimeDelta { secs: -self.secs - secs_diff, nanos }
    }
}

impl Add for TimeDelta {
    type Output = TimeDelta;

    fn add(self, rhs: TimeDelta) -> TimeDelta {
        self.checked_add(rhs).expect("`TimeDelta + TimeDelta` overflowed")
    }
}

impl Sub for TimeDelta {
    type Output = TimeDelta;

    fn sub(self, rhs: TimeDelta) -> TimeDelta {
        self.checked_sub(rhs).expect("`TimeDelta - TimeDelta` overflowed")
    }
}

impl AddAssign for TimeDelta {
    fn add_assign(&mut self, rhs: TimeDelta) {
        let new = self.checked_add(rhs).expect("`TimeDelta + TimeDelta` overflowed");
        *self = new;
    }
}

impl SubAssign for TimeDelta {
    fn sub_assign(&mut self, rhs: TimeDelta) {
        let new = self.checked_sub(rhs).expect("`TimeDelta - TimeDelta` overflowed");
        *self = new;
    }
}

impl Mul<i32> for TimeDelta {
    type Output = TimeDelta;

    fn mul(self, rhs: i32) -> TimeDelta {
        // Multiply nanoseconds as i64, because it cannot overflow that way.
        let total_nanos = self.nanos as i64 * rhs as i64;
        let (extra_secs, nanos) = div_mod_floor_64(total_nanos, NANOS_PER_SEC as i64);
        let secs = self.secs * rhs as i64 + extra_secs;
        TimeDelta { secs, nanos: nanos as i32 }
    }
}

impl Div<i32> for TimeDelta {
    type Output = TimeDelta;

    fn div(self, rhs: i32) -> TimeDelta {
        let mut secs = self.secs / rhs as i64;
        let carry = self.secs - secs * rhs as i64;
        let extra_nanos = carry * NANOS_PER_SEC as i64 / rhs as i64;
        let mut nanos = self.nanos / rhs + extra_nanos as i32;
        if nanos >= NANOS_PER_SEC {
            nanos -= NANOS_PER_SEC;
            secs += 1;
        }
        if nanos < 0 {
            nanos += NANOS_PER_SEC;
            secs -= 1;
        }
        TimeDelta { secs, nanos }
    }
}

impl<'a> core::iter::Sum<&'a TimeDelta> for TimeDelta {
    fn sum<I: Iterator<Item = &'a TimeDelta>>(iter: I) -> TimeDelta {
        iter.fold(TimeDelta::zero(), |acc, x| acc + *x)
    }
}

impl core::iter::Sum<TimeDelta> for TimeDelta {
    fn sum<I: Iterator<Item = TimeDelta>>(iter: I) -> TimeDelta {
        iter.fold(TimeDelta::zero(), |acc, x| acc + x)
    }
}

impl fmt::Display for TimeDelta {
    /// Format a `TimeDelta` using the [ISO 8601] format
    ///
    /// [ISO 8601]: https://en.wikipedia.org/wiki/ISO_8601#Durations
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // technically speaking, negative duration is not valid ISO 8601,
        // but we need to print it anyway.
        let (abs, sign) = if self.secs < 0 { (-*self, "-") } else { (*self, "") };

        write!(f, "{}P", sign)?;
        // Plenty of ways to encode an empty string. `P0D` is short and not too strange.
        if abs.secs == 0 && abs.nanos == 0 {
            return f.write_str("0D");
        }

        f.write_fmt(format_args!("T{}", abs.secs))?;

        if abs.nanos > 0 {
            // Count the number of significant digits, while removing all trailing zero's.
            let mut figures = 9usize;
            let mut fraction_digits = abs.nanos;
            loop {
                let div = fraction_digits / 10;
                let last_digit = fraction_digits % 10;
                if last_digit != 0 {
                    break;
                }
                fraction_digits = div;
                figures -= 1;
            }
            f.write_fmt(format_args!(".{:01$}", fraction_digits, figures))?;
        }
        f.write_str("S")?;
        Ok(())
    }
}

/// Represents error when converting `TimeDelta` to/from a standard library
/// implementation
///
/// The `std::time::Duration` supports a range from zero to `u64::MAX`
/// *seconds*, while this module supports signed range of up to
/// `i64::MAX` of *milliseconds*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfRangeError(());

impl fmt::Display for OutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Source duration value is out of range for the target type")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OutOfRangeError {
    #[allow(deprecated)]
    fn description(&self) -> &str {
        "out of range error"
    }
}

#[inline]
const fn div_mod_floor_64(this: i64, other: i64) -> (i64, i64) {
    (this.div_euclid(other), this.rem_euclid(other))
}

#[cfg(all(feature = "arbitrary", feature = "std"))]
impl arbitrary::Arbitrary<'_> for TimeDelta {
    fn arbitrary(u: &mut arbitrary::Unstructured) -> arbitrary::Result<TimeDelta> {
        const MIN_SECS: i64 = -i64::MAX / MILLIS_PER_SEC - 1;
        const MAX_SECS: i64 = i64::MAX / MILLIS_PER_SEC;

        let secs: i64 = u.int_in_range(MIN_SECS..=MAX_SECS)?;
        let nanos: i32 = u.int_in_range(0..=(NANOS_PER_SEC - 1))?;
        let duration = TimeDelta { secs, nanos };

        if duration < MIN || duration > MAX {
            Err(arbitrary::Error::IncorrectFormat)
        } else {
            Ok(duration)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OutOfRangeError;
    use super::{TimeDelta, MAX, MIN};
    use crate::{expect, ok, Error};
    use core::time::Duration;

    #[test]
    fn test_timedelta_max_value() {
        const MAX: TimeDelta = TimeDelta::max_value();
        dbg!(MAX.num_weeks());
        assert!(MAX.num_weeks() > i32::MAX as i64);
        dbg!(MAX.num_days());
        assert!(MAX.num_days() > i32::MAX as i64);
        dbg!(MAX.num_hours());
        assert!(MAX.num_hours() > i32::MAX as i64);
    }

    #[test]
    fn test_duration() {
        let days = TimeDelta::days;
        let seconds = TimeDelta::seconds;

        assert!(seconds(1) != TimeDelta::zero());
        assert_eq!(seconds(1) + seconds(2), seconds(3));
        assert_eq!(seconds(86_399) + seconds(4), days(1) + seconds(3));
        assert_eq!(days(10) - seconds(1000), seconds(863_000));
        assert_eq!(days(10) - seconds(1_000_000), seconds(-136_000));
        assert_eq!(
            days(2) + seconds(86_399) + TimeDelta::nanoseconds(1_234_567_890),
            days(3) + TimeDelta::nanoseconds(234_567_890)
        );
        assert_eq!(-days(3), days(-3));
        assert_eq!(-(days(3) + seconds(70)), days(-4) + seconds(86_400 - 70));

        let mut d = TimeDelta::default();
        d += TimeDelta::minutes(1);
        d -= seconds(30);
        assert_eq!(d, seconds(30));
    }

    #[test]
    fn test_duration_num_days() {
        assert_eq!(TimeDelta::zero().num_days(), 0);
        assert_eq!(TimeDelta::days(1).num_days(), 1);
        assert_eq!(TimeDelta::days(-1).num_days(), -1);
        assert_eq!(TimeDelta::seconds(86_399).num_days(), 0);
        assert_eq!(TimeDelta::seconds(86_401).num_days(), 1);
        assert_eq!(TimeDelta::seconds(-86_399).num_days(), 0);
        assert_eq!(TimeDelta::seconds(-86_401).num_days(), -1);
        assert_eq!(TimeDelta::days(i32::MAX).num_days(), i32::MAX as i64);
        assert_eq!(TimeDelta::days(i32::MIN).num_days(), i32::MIN as i64);
    }

    #[test]
    fn test_duration_num_seconds() {
        assert_eq!(TimeDelta::zero().num_seconds(), 0);
        assert_eq!(TimeDelta::seconds(1).num_seconds(), 1);
        assert_eq!(TimeDelta::seconds(-1).num_seconds(), -1);
        assert_eq!(TimeDelta::milliseconds(999).unwrap().num_seconds(), 0);
        assert_eq!(TimeDelta::milliseconds(1001).unwrap().num_seconds(), 1);
        assert_eq!(TimeDelta::milliseconds(-999).unwrap().num_seconds(), 0);
        assert_eq!(TimeDelta::milliseconds(-1001).unwrap().num_seconds(), -1);
    }

    #[test]
    fn test_duration_num_milliseconds() {
        assert_eq!(TimeDelta::zero().num_milliseconds(), 0);
        assert_eq!(TimeDelta::milliseconds(1).unwrap().num_milliseconds(), 1);
        assert_eq!(TimeDelta::milliseconds(-1).unwrap().num_milliseconds(), -1);
        assert_eq!(TimeDelta::microseconds(999).num_milliseconds(), 0);
        assert_eq!(TimeDelta::microseconds(1001).num_milliseconds(), 1);
        assert_eq!(TimeDelta::microseconds(-999).num_milliseconds(), 0);
        assert_eq!(TimeDelta::microseconds(-1001).num_milliseconds(), -1);
    }

    #[test]
    fn test_duration_milliseconds_max_allowed() {
        // The maximum number of milliseconds acceptable through the constructor is
        // equal to the number that can be stored in a TimeDelta.
        let duration = TimeDelta::milliseconds(i64::MAX).unwrap();
        assert_eq!(duration.num_milliseconds(), i64::MAX);
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            i64::MAX as i128 * 1_000_000
        );
    }

    #[test]
    fn test_duration_milliseconds_max_overflow() {
        // Here we ensure that trying to add one millisecond to the maximum storable
        // value will fail.
        assert!(TimeDelta::milliseconds(i64::MAX)
            .unwrap()
            .checked_add(TimeDelta::milliseconds(1).unwrap())
            .is_none());
    }

    #[test]
    fn test_duration_milliseconds_min_allowed() {
        // The minimum number of milliseconds acceptable through the constructor is
        // not equal to the number that can be stored in a TimeDelta - there is a
        // difference of one (i64::MIN vs -i64::MAX).
        let duration = TimeDelta::milliseconds(-i64::MAX).unwrap();
        assert_eq!(duration.num_milliseconds(), -i64::MAX);
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            -i64::MAX as i128 * 1_000_000
        );
    }

    #[test]
    fn test_duration_milliseconds_min_underflow() {
        // Here we ensure that trying to subtract one millisecond from the minimum
        // storable value will fail.
        assert!(TimeDelta::milliseconds(-i64::MAX)
            .unwrap()
            .checked_sub(TimeDelta::milliseconds(1).unwrap())
            .is_none());
    }

    #[test]
    fn test_duration_num_microseconds() {
        assert_eq!(TimeDelta::zero().num_microseconds(), Ok(0));
        assert_eq!(TimeDelta::microseconds(1).num_microseconds(), Ok(1));
        assert_eq!(TimeDelta::microseconds(-1).num_microseconds(), Ok(-1));
        assert_eq!(TimeDelta::nanoseconds(999).num_microseconds(), Ok(0));
        assert_eq!(TimeDelta::nanoseconds(1001).num_microseconds(), Ok(1));
        assert_eq!(TimeDelta::nanoseconds(-999).num_microseconds(), Ok(0));
        assert_eq!(TimeDelta::nanoseconds(-1001).num_microseconds(), Ok(-1));

        // overflow checks
        assert_eq!(TimeDelta::max_value().num_microseconds(), Err(Error::OutOfRange));
        assert_eq!(TimeDelta::min_value().num_microseconds(), Err(Error::OutOfRange));
    }
    #[test]
    fn test_duration_microseconds_max_allowed() {
        // The number of microseconds acceptable through the constructor is far
        // fewer than the number that can actually be stored in a TimeDelta, so this
        // is not a particular insightful test.
        let duration = TimeDelta::microseconds(i64::MAX);
        assert_eq!(duration.num_microseconds(), Ok(i64::MAX));
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            i64::MAX as i128 * 1_000
        );
        // Here we create a TimeDelta with the maximum possible number of
        // microseconds by creating a TimeDelta with the maximum number of
        // milliseconds and then checking that the number of microseconds matches
        // the storage limit.
        let duration = TimeDelta::milliseconds(i64::MAX).unwrap();
        assert_eq!(duration.num_microseconds(), Err(Error::OutOfRange));
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            i64::MAX as i128 * 1_000_000
        );
    }
    #[test]
    fn test_duration_microseconds_max_overflow() {
        // This test establishes that a TimeDelta can store more microseconds than
        // are representable through the return of duration.num_microseconds().
        let duration = TimeDelta::microseconds(i64::MAX) + TimeDelta::microseconds(1);
        assert_eq!(duration.num_microseconds(), Err(Error::OutOfRange));
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            (i64::MAX as i128 + 1) * 1_000
        );
        // Here we ensure that trying to add one microsecond to the maximum storable
        // value will fail.
        assert!(TimeDelta::milliseconds(i64::MAX)
            .unwrap()
            .checked_add(TimeDelta::microseconds(1))
            .is_none());
    }
    #[test]
    fn test_duration_microseconds_min_allowed() {
        // The number of microseconds acceptable through the constructor is far
        // fewer than the number that can actually be stored in a TimeDelta, so this
        // is not a particular insightful test.
        let duration = TimeDelta::microseconds(i64::MIN);
        assert_eq!(duration.num_microseconds(), Ok(i64::MIN));
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            i64::MIN as i128 * 1_000
        );
        // Here we create a TimeDelta with the minimum possible number of
        // microseconds by creating a TimeDelta with the minimum number of
        // milliseconds and then checking that the number of microseconds matches
        // the storage limit.
        let duration = TimeDelta::milliseconds(-i64::MAX).unwrap();
        assert_eq!(duration.num_microseconds(), Err(Error::OutOfRange));
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            -i64::MAX as i128 * 1_000_000
        );
    }
    #[test]
    fn test_duration_microseconds_min_underflow() {
        // This test establishes that a TimeDelta can store more microseconds than
        // are representable through the return of duration.num_microseconds().
        let duration = TimeDelta::microseconds(i64::MIN) - TimeDelta::microseconds(1);
        assert_eq!(duration.num_microseconds(), Err(Error::OutOfRange));
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            (i64::MIN as i128 - 1) * 1_000
        );
        // Here we ensure that trying to subtract one microsecond from the minimum
        // storable value will fail.
        assert!(TimeDelta::milliseconds(-i64::MAX)
            .unwrap()
            .checked_sub(TimeDelta::microseconds(1))
            .is_none());
    }

    #[test]
    fn test_duration_num_nanoseconds() {
        assert_eq!(TimeDelta::zero().num_nanoseconds(), Some(0));
        assert_eq!(TimeDelta::nanoseconds(1).num_nanoseconds(), Some(1));
        assert_eq!(TimeDelta::nanoseconds(-1).num_nanoseconds(), Some(-1));

        // overflow checks
        assert_eq!(TimeDelta::max_value().num_nanoseconds(), None);
        assert_eq!(TimeDelta::min_value().num_nanoseconds(), None);
    }
    #[test]
    fn test_duration_nanoseconds_max_allowed() {
        // The number of nanoseconds acceptable through the constructor is far fewer
        // than the number that can actually be stored in a TimeDelta, so this is not
        // a particular insightful test.
        let duration = TimeDelta::nanoseconds(i64::MAX);
        assert_eq!(duration.num_nanoseconds(), Some(i64::MAX));
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            i64::MAX as i128
        );
        // Here we create a TimeDelta with the maximum possible number of nanoseconds
        // by creating a TimeDelta with the maximum number of milliseconds and then
        // checking that the number of nanoseconds matches the storage limit.
        let duration = TimeDelta::milliseconds(i64::MAX).unwrap();
        assert!(duration.num_nanoseconds().is_none());
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            i64::MAX as i128 * 1_000_000
        );
    }

    #[test]
    fn test_duration_nanoseconds_max_overflow() {
        // This test establishes that a TimeDelta can store more nanoseconds than are
        // representable through the return of duration.num_nanoseconds().
        let duration = TimeDelta::nanoseconds(i64::MAX) + TimeDelta::nanoseconds(1);
        assert!(duration.num_nanoseconds().is_none());
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            i64::MAX as i128 + 1
        );
        // Here we ensure that trying to add one nanosecond to the maximum storable
        // value will fail.
        assert!(TimeDelta::milliseconds(i64::MAX)
            .unwrap()
            .checked_add(TimeDelta::nanoseconds(1))
            .is_none());
    }

    #[test]
    fn test_duration_nanoseconds_min_allowed() {
        // The number of nanoseconds acceptable through the constructor is far fewer
        // than the number that can actually be stored in a TimeDelta, so this is not
        // a particular insightful test.
        let duration = TimeDelta::nanoseconds(i64::MIN);
        assert_eq!(duration.num_nanoseconds(), Some(i64::MIN));
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            i64::MIN as i128
        );
        // Here we create a TimeDelta with the minimum possible number of nanoseconds
        // by creating a TimeDelta with the minimum number of milliseconds and then
        // checking that the number of nanoseconds matches the storage limit.
        let duration = TimeDelta::milliseconds(-i64::MAX).unwrap();
        assert!(duration.num_nanoseconds().is_none());
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            -i64::MAX as i128 * 1_000_000
        );
    }

    #[test]
    fn test_duration_nanoseconds_min_underflow() {
        // This test establishes that a TimeDelta can store more nanoseconds than are
        // representable through the return of duration.num_nanoseconds().
        let duration = TimeDelta::nanoseconds(i64::MIN) - TimeDelta::nanoseconds(1);
        assert!(duration.num_nanoseconds().is_none());
        assert_eq!(
            duration.secs as i128 * 1_000_000_000 + duration.nanos as i128,
            i64::MIN as i128 - 1
        );
        // Here we ensure that trying to subtract one nanosecond from the minimum
        // storable value will fail.
        assert!(TimeDelta::milliseconds(-i64::MAX)
            .unwrap()
            .checked_sub(TimeDelta::nanoseconds(1))
            .is_none());
    }

    #[test]
    fn test_max() {
        assert_eq!(
            MAX.secs as i128 * 1_000_000_000 + MAX.nanos as i128,
            i64::MAX as i128 * 1_000_000
        );
        assert_eq!(MAX, TimeDelta::milliseconds(i64::MAX).unwrap());
        assert_eq!(MAX.num_milliseconds(), i64::MAX);
        assert_eq!(MAX.num_microseconds(), Err(Error::OutOfRange));
        assert_eq!(MAX.num_nanoseconds(), None);
    }

    #[test]
    fn test_min() {
        assert_eq!(
            MIN.secs as i128 * 1_000_000_000 + MIN.nanos as i128,
            -i64::MAX as i128 * 1_000_000
        );
        assert_eq!(MIN, TimeDelta::milliseconds(-i64::MAX).unwrap());
        assert_eq!(MIN.num_milliseconds(), -i64::MAX);
        assert_eq!(MIN.num_microseconds(), Err(Error::OutOfRange));
        assert_eq!(MIN.num_nanoseconds(), None);
    }

    #[test]
    fn test_duration_ord() {
        let milliseconds = |ms| TimeDelta::milliseconds(ms).unwrap();

        assert!(milliseconds(1) < milliseconds(2));
        assert!(milliseconds(2) > milliseconds(1));
        assert!(milliseconds(-1) > milliseconds(-2));
        assert!(milliseconds(-2) < milliseconds(-1));
        assert!(milliseconds(-1) < milliseconds(1));
        assert!(milliseconds(1) > milliseconds(-1));
        assert!(milliseconds(0) < milliseconds(1));
        assert!(milliseconds(0) > milliseconds(-1));
        assert!(milliseconds(1_001) < milliseconds(1_002));
        assert!(milliseconds(-1_001) > milliseconds(-1_002));
        assert!(TimeDelta::nanoseconds(1_234_567_890) < TimeDelta::nanoseconds(1_234_567_891));
        assert!(TimeDelta::nanoseconds(-1_234_567_890) > TimeDelta::nanoseconds(-1_234_567_891));
        assert!(milliseconds(i64::MAX) > milliseconds(i64::MAX - 1));
        assert!(milliseconds(-i64::MAX) < milliseconds(-i64::MAX + 1));
    }

    #[test]
    fn test_duration_checked_ops() {
        let milliseconds = |ms| TimeDelta::milliseconds(ms).unwrap();

        assert_eq!(
            milliseconds(i64::MAX).checked_add(milliseconds(0)),
            Some(milliseconds(i64::MAX))
        );
        assert_eq!(
            milliseconds(i64::MAX - 1).checked_add(TimeDelta::microseconds(999)),
            Some(milliseconds(i64::MAX - 2) + TimeDelta::microseconds(1999))
        );
        assert!(milliseconds(i64::MAX).checked_add(TimeDelta::microseconds(1000)).is_none());
        assert!(milliseconds(i64::MAX).checked_add(TimeDelta::nanoseconds(1)).is_none());

        assert_eq!(
            milliseconds(-i64::MAX).checked_sub(milliseconds(0)),
            Some(milliseconds(-i64::MAX))
        );
        assert_eq!(
            milliseconds(-i64::MAX + 1).checked_sub(TimeDelta::microseconds(999)),
            Some(milliseconds(-i64::MAX + 2) - TimeDelta::microseconds(1999))
        );
        assert!(milliseconds(-i64::MAX).checked_sub(milliseconds(1)).is_none());
        assert!(milliseconds(-i64::MAX).checked_sub(TimeDelta::nanoseconds(1)).is_none());
    }

    #[test]
    fn test_duration_abs() {
        let milliseconds = |ms| TimeDelta::milliseconds(ms).unwrap();

        assert_eq!(milliseconds(1300).abs(), milliseconds(1300));
        assert_eq!(milliseconds(1000).abs(), milliseconds(1000));
        assert_eq!(milliseconds(300).abs(), milliseconds(300));
        assert_eq!(milliseconds(0).abs(), milliseconds(0));
        assert_eq!(milliseconds(-300).abs(), milliseconds(300));
        assert_eq!(milliseconds(-700).abs(), milliseconds(700));
        assert_eq!(milliseconds(-1000).abs(), milliseconds(1000));
        assert_eq!(milliseconds(-1300).abs(), milliseconds(1300));
        assert_eq!(milliseconds(-1700).abs(), milliseconds(1700));
        assert_eq!(milliseconds(-i64::MAX).abs(), milliseconds(i64::MAX));
    }

    #[test]
    #[allow(clippy::erasing_op)]
    fn test_duration_mul() {
        assert_eq!(TimeDelta::zero() * i32::MAX, TimeDelta::zero());
        assert_eq!(TimeDelta::zero() * i32::MIN, TimeDelta::zero());
        assert_eq!(TimeDelta::nanoseconds(1) * 0, TimeDelta::zero());
        assert_eq!(TimeDelta::nanoseconds(1) * 1, TimeDelta::nanoseconds(1));
        assert_eq!(TimeDelta::nanoseconds(1) * 1_000_000_000, TimeDelta::seconds(1));
        assert_eq!(TimeDelta::nanoseconds(1) * -1_000_000_000, -TimeDelta::seconds(1));
        assert_eq!(-TimeDelta::nanoseconds(1) * 1_000_000_000, -TimeDelta::seconds(1));
        assert_eq!(
            TimeDelta::nanoseconds(30) * 333_333_333,
            TimeDelta::seconds(10) - TimeDelta::nanoseconds(10)
        );
        assert_eq!(
            (TimeDelta::nanoseconds(1) + TimeDelta::seconds(1) + TimeDelta::days(1)) * 3,
            TimeDelta::nanoseconds(3) + TimeDelta::seconds(3) + TimeDelta::days(3)
        );
        assert_eq!(TimeDelta::milliseconds(1500).unwrap() * -2, TimeDelta::seconds(-3));
        assert_eq!(TimeDelta::milliseconds(-1500).unwrap() * 2, TimeDelta::seconds(-3));
    }

    #[test]
    fn test_duration_div() {
        assert_eq!(TimeDelta::zero() / i32::MAX, TimeDelta::zero());
        assert_eq!(TimeDelta::zero() / i32::MIN, TimeDelta::zero());
        assert_eq!(TimeDelta::nanoseconds(123_456_789) / 1, TimeDelta::nanoseconds(123_456_789));
        assert_eq!(TimeDelta::nanoseconds(123_456_789) / -1, -TimeDelta::nanoseconds(123_456_789));
        assert_eq!(-TimeDelta::nanoseconds(123_456_789) / -1, TimeDelta::nanoseconds(123_456_789));
        assert_eq!(-TimeDelta::nanoseconds(123_456_789) / 1, -TimeDelta::nanoseconds(123_456_789));
        assert_eq!(TimeDelta::seconds(1) / 3, TimeDelta::nanoseconds(333_333_333));
        assert_eq!(TimeDelta::seconds(4) / 3, TimeDelta::nanoseconds(1_333_333_333));
        assert_eq!(TimeDelta::seconds(-1) / 2, TimeDelta::milliseconds(-500).unwrap());
        assert_eq!(TimeDelta::seconds(1) / -2, TimeDelta::milliseconds(-500).unwrap());
        assert_eq!(TimeDelta::seconds(-1) / -2, TimeDelta::milliseconds(500).unwrap());
        assert_eq!(TimeDelta::seconds(-4) / 3, TimeDelta::nanoseconds(-1_333_333_333));
        assert_eq!(TimeDelta::seconds(-4) / -3, TimeDelta::nanoseconds(1_333_333_333));
    }

    #[test]
    fn test_duration_sum() {
        let duration_list_1 = [TimeDelta::zero(), TimeDelta::seconds(1)];
        let sum_1: TimeDelta = duration_list_1.iter().sum();
        assert_eq!(sum_1, TimeDelta::seconds(1));

        let duration_list_2 = [
            TimeDelta::zero(),
            TimeDelta::seconds(1),
            TimeDelta::seconds(6),
            TimeDelta::seconds(10),
        ];
        let sum_2: TimeDelta = duration_list_2.iter().sum();
        assert_eq!(sum_2, TimeDelta::seconds(17));

        let duration_arr = [
            TimeDelta::zero(),
            TimeDelta::seconds(1),
            TimeDelta::seconds(6),
            TimeDelta::seconds(10),
        ];
        let sum_3: TimeDelta = duration_arr.into_iter().sum();
        assert_eq!(sum_3, TimeDelta::seconds(17));
    }

    #[test]
    fn test_duration_fmt() {
        assert_eq!(TimeDelta::zero().to_string(), "P0D");
        assert_eq!(TimeDelta::days(42).to_string(), "PT3628800S");
        assert_eq!(TimeDelta::days(-42).to_string(), "-PT3628800S");
        assert_eq!(TimeDelta::seconds(42).to_string(), "PT42S");
        assert_eq!(TimeDelta::milliseconds(42).unwrap().to_string(), "PT0.042S");
        assert_eq!(TimeDelta::microseconds(42).to_string(), "PT0.000042S");
        assert_eq!(TimeDelta::nanoseconds(42).to_string(), "PT0.000000042S");
        assert_eq!(
            (TimeDelta::days(7) + TimeDelta::milliseconds(6543).unwrap()).to_string(),
            "PT604806.543S"
        );
        assert_eq!(TimeDelta::seconds(-86_401).to_string(), "-PT86401S");
        assert_eq!(TimeDelta::nanoseconds(-1).to_string(), "-PT0.000000001S");

        // the format specifier should have no effect on `TimeDelta`
        assert_eq!(
            format!("{:30}", TimeDelta::days(1) + TimeDelta::milliseconds(2345).unwrap()),
            "PT86402.345S"
        );
    }

    #[test]
    fn test_to_std() {
        assert_eq!(TimeDelta::seconds(1).to_std(), Ok(Duration::new(1, 0)));
        assert_eq!(TimeDelta::seconds(86_401).to_std(), Ok(Duration::new(86_401, 0)));
        assert_eq!(
            TimeDelta::milliseconds(123).unwrap().to_std(),
            Ok(Duration::new(0, 123_000_000))
        );
        assert_eq!(
            TimeDelta::milliseconds(123_765).unwrap().to_std(),
            Ok(Duration::new(123, 765_000_000))
        );
        assert_eq!(TimeDelta::nanoseconds(777).to_std(), Ok(Duration::new(0, 777)));
        assert_eq!(MAX.to_std(), Ok(Duration::new(9_223_372_036_854_775, 807_000_000)));
        assert_eq!(TimeDelta::seconds(-1).to_std(), Err(OutOfRangeError(())));
        assert_eq!(TimeDelta::milliseconds(-1).unwrap().to_std(), Err(OutOfRangeError(())));
    }

    #[test]
    fn test_from_std() {
        assert_eq!(Ok(TimeDelta::seconds(1)), TimeDelta::from_std(Duration::new(1, 0)));
        assert_eq!(Ok(TimeDelta::seconds(86_401)), TimeDelta::from_std(Duration::new(86_401, 0)));
        assert_eq!(
            Ok(TimeDelta::milliseconds(123).unwrap()),
            TimeDelta::from_std(Duration::new(0, 123_000_000))
        );
        assert_eq!(
            Ok(TimeDelta::milliseconds(123_765).unwrap()),
            TimeDelta::from_std(Duration::new(123, 765_000_000))
        );
        assert_eq!(Ok(TimeDelta::nanoseconds(777)), TimeDelta::from_std(Duration::new(0, 777)));
        assert_eq!(Ok(MAX), TimeDelta::from_std(Duration::new(9_223_372_036_854_775, 807_000_000)));
        assert_eq!(
            TimeDelta::from_std(Duration::new(9_223_372_036_854_776, 0)),
            Err(OutOfRangeError(()))
        );
        assert_eq!(
            TimeDelta::from_std(Duration::new(9_223_372_036_854_775, 807_000_001)),
            Err(OutOfRangeError(()))
        );
    }

    #[test]
    fn test_duration_const() {
        const ONE_WEEK: TimeDelta = TimeDelta::weeks(1);
        const ONE_DAY: TimeDelta = TimeDelta::days(1);
        const ONE_HOUR: TimeDelta = TimeDelta::hours(1);
        const ONE_MINUTE: TimeDelta = TimeDelta::minutes(1);
        const ONE_SECOND: TimeDelta = TimeDelta::seconds(1);
        const ONE_MILLI: TimeDelta = expect(ok!(TimeDelta::milliseconds(1)), "");
        const ONE_MICRO: TimeDelta = TimeDelta::microseconds(1);
        const ONE_NANO: TimeDelta = TimeDelta::nanoseconds(1);
        let combo: TimeDelta = ONE_WEEK
            + ONE_DAY
            + ONE_HOUR
            + ONE_MINUTE
            + ONE_SECOND
            + ONE_MILLI
            + ONE_MICRO
            + ONE_NANO;

        assert!(ONE_WEEK != TimeDelta::zero());
        assert!(ONE_DAY != TimeDelta::zero());
        assert!(ONE_HOUR != TimeDelta::zero());
        assert!(ONE_MINUTE != TimeDelta::zero());
        assert!(ONE_SECOND != TimeDelta::zero());
        assert!(ONE_MILLI != TimeDelta::zero());
        assert!(ONE_MICRO != TimeDelta::zero());
        assert!(ONE_NANO != TimeDelta::zero());
        assert_eq!(
            combo,
            TimeDelta::seconds(86400 * 7 + 86400 + 3600 + 60 + 1)
                + TimeDelta::nanoseconds(1 + 1_000 + 1_000_000)
        );
    }

    #[test]
    #[cfg(feature = "rkyv-validation")]
    fn test_rkyv_validation() {
        let duration = TimeDelta::seconds(1);
        let bytes = rkyv::to_bytes::<_, 16>(&duration).unwrap();
        assert_eq!(rkyv::from_bytes::<TimeDelta>(&bytes).unwrap(), duration);
    }
}

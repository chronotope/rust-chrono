use core::fmt;
use core::num::NonZeroU64;

use crate::{expect, try_opt};

/// Duration type capable of expressing a duration as a combination of multiple units.
///
/// A duration can be expressed as a combination of four components: *months*, *days*, *minutes*,
/// and *seconds and nanoseconds*. This type is compatible with the definition of an ISO 8601
/// duration.
///
/// # Components
///
/// The *months* and *days* components are called components with a "nominal" durations, and can
/// also be used to express years and weeks respectively.
///
/// The *minutes*, and *seconds and nanoseconds* components are called components with an "accurate"
/// duration. Hours can be expressed with the *minutes* component. The definition of a minute is not
/// exactly 60 seconds; it can also be 61 or 59 in the presence of leap seconds.
///
/// To keep operations with a `CalendarDate` sane a large value for the *seconds* component is
/// mutually exclusive with the presence of a *minutes* component. If the *minutes* component is set
/// the *seconds* component is limited to 60 seconds.
///
/// **Note**: The distinction between *minutes and seconds* components and only a *seconds*
/// component prepares the `CalendarDate` type for working with leap seconds accurately.
/// However they currently behave the same because full support for leap seconds is not yet
/// implemented in chrono.
///
/// | component           | limit                    |
/// |---------------------|--------------------------|
/// | months              | `u32::MAX`               |
/// | days                | `u32::MAX`               |
/// | minutes and seconds | `u64::MAX >> 8` and `60` |
/// | seconds             | `u64::MAX >> 2`          |
/// | nanoseconds         | `999_999_999`            |
///
/// # Examples
///
/// ```
/// # use chrono::CalendarDuration;
/// // Encoding 1½ year as a duration of 18 months:
/// let _duration = CalendarDuration::new().with_months(18);
///
/// // Encoding 1½ year as a duration of 12 months and 183 days (366 / 2):
/// let _duration = CalendarDuration::new().with_months(12).with_days(183);
///
/// // Encoding 1½ year as a duration of 548 days (365 + 366 / 2):
/// let _duration = CalendarDuration::new().with_days(548);
///
/// // Encoding 1½ year as a duration in seconds:
/// let _duration = CalendarDuration::new().with_seconds(548 * 24 * 60 * 60).unwrap();
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CalendarDuration {
    // Components with a nominal duration
    months: u32,
    days: u32,
    // Components with an accurate duration
    mins_and_secs: MinutesAndSeconds,
    nanos: u32,
}

impl Default for CalendarDuration {
    fn default() -> Self {
        CalendarDuration::new()
    }
}

impl fmt::Debug for CalendarDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (mins, secs) = self.mins_and_secs();
        let mut builder = f.debug_struct("CalendarDuration");
        if self.months != 0 {
            builder.field("months", &self.months);
        }
        if self.days != 0 {
            builder.field("days", &self.days);
        }
        if mins != 0 {
            builder.field("minutes", &mins);
        }
        if secs != 0 {
            builder.field("seconds", &secs);
        }
        if self.nanos != 0 {
            builder.field("nanos", &self.nanos);
        }
        builder.finish()
    }
}

impl CalendarDuration {
    /// Create a new duration initialized to `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use chrono::CalendarDuration;
    /// // Duration in calendar months and days.
    /// let _ = CalendarDuration::new().with_months(15).with_days(5);
    /// // Duration in calendar days, and 3 hours.
    /// let _ = CalendarDuration::new().with_days(5).with_hms(3, 0, 0).unwrap();
    /// // Duration that will precisely count the number of seconds, including leap seconds.
    /// let _ = CalendarDuration::new().with_seconds(23_456).unwrap().with_nanos(789).unwrap();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            months: 0,
            days: 0,
            mins_and_secs: expect(MinutesAndSeconds::from_seconds(0), "in range"),
            nanos: 0,
        }
    }

    /// Set the months component of this duration to `months`.
    #[must_use]
    pub const fn with_months(mut self, months: u32) -> Self {
        self.months = months;
        self
    }

    /// Set the days component of this duration to `days`.
    #[must_use]
    pub const fn with_days(mut self, days: u32) -> Self {
        self.days = days;
        self
    }

    /// Sets the accurate component of this duration to a value in minutes and seconds.
    ///
    /// The definition of a minute is not exactly 60 seconds; it can also be 61 or 59 in the
    /// presence of leap seconds.
    ///
    /// Durations made with this method are the closest thing to working like leap seconds don't
    /// exists.
    ///
    /// # Errors
    ///
    /// Returns `None`:
    /// - if `seconds` exceeds 60
    /// - if the value of the minutes component (`hours * 60 + minutes`) would be out of range.
    pub const fn with_hms(mut self, hours: u64, minutes: u64, seconds: u8) -> Option<Self> {
        let minutes = try_opt!(try_opt!(hours.checked_mul(60)).checked_add(minutes));
        self.mins_and_secs =
            try_opt!(MinutesAndSeconds::from_minutes_and_seconds(minutes, seconds as u64));
        Some(self)
    }

    /// Sets the accurate component of this duration to a value in seconds.
    ///
    /// This duration will count an exact number of seconds, which includes the counting of leap
    /// seconds.
    ///
    /// The minutes component will be set to zero.
    ///
    /// # Errors
    ///
    /// Returns `None` if the value of the seconds component would be out of range.
    #[must_use]
    pub const fn with_seconds(mut self, seconds: u64) -> Option<Self> {
        self.mins_and_secs = try_opt!(MinutesAndSeconds::from_seconds(seconds));
        Some(self)
    }

    /// Sets the nanoseconds component of this duration to `nanos`.
    ///
    /// # Errors
    ///
    /// Returns `None` if `nanos` exceeds 999_999_999.
    pub const fn with_nanos(mut self, nanos: u32) -> Option<Self> {
        if nanos >= 1_000_000_000 {
            return None;
        }
        self.nanos = nanos;
        Some(self)
    }

    /// Returns the `months` component of this calendar duration.
    pub const fn months(&self) -> u32 {
        self.months
    }

    /// Returns the `days` component of this calendar duration.
    pub const fn days(&self) -> u32 {
        self.days
    }

    /// Returns the `minutes` and `seconds` components of this calendar duration.
    pub const fn mins_and_secs(&self) -> (u64, u64) {
        self.mins_and_secs.mins_and_secs()
    }

    /// Returns the `nanos` component of this calendar duration.
    pub const fn nanos(&self) -> u32 {
        self.nanos
    }

    /// Returns `true` if this `CalendarDuration` spans no time.
    pub const fn is_zero(&self) -> bool {
        let (mins, secs) = self.mins_and_secs();
        self.months == 0 && self.days == 0 && mins == 0 && secs == 0 && self.nanos == 0
    }
}

/// Type to encode either seconds, or minutes and up to 60 seconds.
///
/// We don't allow having both an `u64` with minutes and an `u64` with seconds.
/// Having two components in a `CalendarDuration` whose only subtle difference is how they behave
/// around leap seconds will make the type unintuitive.
///
/// # Encoding
///
/// - Seconds:                                     `seconds << 2 | 0b10`
/// - Minutes and up to 60 seconds: `minutes << 8 | seconds << 2 | 0b11`
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct MinutesAndSeconds(NonZeroU64);

impl MinutesAndSeconds {
    const fn from_seconds(secs: u64) -> Option<Self> {
        if secs >= (1 << 62) {
            return None;
        }
        Some(Self(expect(NonZeroU64::new(secs << 2 | 0b10), "can't fail, non-zero")))
    }

    const fn from_minutes_and_seconds(mins: u64, secs: u64) -> Option<Self> {
        if mins >= (1 << 56) || secs > 60 {
            return None;
        }
        // The `(mins > 0) as u64` causes a value without minutes to have the same encoding as
        // one created with `from_seconds`.
        let val = mins << 8 | secs << 2 | (mins > 0) as u64 | 0b10;
        Some(Self(expect(NonZeroU64::new(val), "can't fail, non-zero")))
    }

    /// Returns the `minutes` and `seconds` components.
    const fn mins_and_secs(&self) -> (u64, u64) {
        let value = self.0.get();
        match value & 0b01 == 0 {
            true => (0, value >> 2),
            false => (value >> 8, (value >> 2) & 0b11_1111),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CalendarDuration;

    #[test]
    fn test_basic_functionality() {
        #[track_caller]
        fn compare(d: CalendarDuration, months: u32, days: u32, mins: u64, secs: u64, nanos: u32) {
            assert_eq!(d.months(), months);
            assert_eq!(d.days(), days);
            assert_eq!((d.mins_and_secs()), (mins, secs));
            assert_eq!(d.nanos(), nanos);
        }

        let duration = CalendarDuration::new()
            .with_months(1)
            .with_days(2)
            .with_hms(3, 4, 5)
            .unwrap()
            .with_nanos(6)
            .unwrap();
        compare(duration, 1, 2, 3 * 60 + 4, 5, 6);

        compare(CalendarDuration::new().with_months(18), 18, 0, 0, 0, 0);
        compare(CalendarDuration::new().with_days(15), 0, 15, 0, 0, 0);
        compare(CalendarDuration::new().with_hms(3, 4, 5).unwrap(), 0, 0, 3 * 60 + 4, 5, 0);
        compare(CalendarDuration::new().with_seconds(123_456).unwrap(), 0, 0, 0, 123_456, 0);
        compare(CalendarDuration::new().with_nanos(123_456_789).unwrap(), 0, 0, 0, 0, 123_456_789);

        compare(CalendarDuration::new(), 0, 0, 0, 0, 0);
        assert!(CalendarDuration::new().is_zero());
        assert!(CalendarDuration::default().is_zero());
    }
}

//! Macro's for easy initialization of date and time values.

/// Create a [`NaiveDate`](crate::naive::NaiveDate) with a statically known value.
///
/// Supported formats are 'year-month-day' and 'year-ordinal'.
///
/// The input is checked at compile time.
///
/// Note: rustfmt wants to add spaces around `-` in this macro.
/// For nice formatting use `#[rustfmt::skip::macros(date)]`, or use as `date! {2023-09-08}`
///
/// # Examples
/// ```
/// use chrono::date;
///
/// assert_eq!(date!(2023-09-08), date!(2023-251));
/// ```
#[macro_export]
macro_rules! date {
    ($y:literal-$m:literal-$d:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            const DATE: $crate::NaiveDate = match $crate::NaiveDate::from_ymd_opt($y, $m, $d) {
                Some(d) => d,
                None => panic!("invalid calendar date"),
            };
            DATE
        }
    }};
    ($y:literal-$o:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            const DATE: $crate::NaiveDate = match $crate::NaiveDate::from_yo_opt($y, $o) {
                Some(d) => d,
                None => panic!("invalid ordinal date"),
            };
            DATE
        }
    }};
}

/// Create a [`NaiveTime`](crate::naive::NaiveTime) with a statically known value.
///
/// Supported formats are 'hour:minute' and 'hour:minute:second'.
///
/// The input is checked at compile time.
///
/// # Examples
/// ```
/// use chrono::time;
/// # use chrono::Timelike;
///
/// assert_eq!(time!(7:03), time!(7:03:00));
/// let leap_second = time!(23:59:60);
/// # assert!(leap_second.second() == 59 && leap_second.nanosecond() == 1_000_000_000);
/// ```
#[macro_export]
macro_rules! time {
    ($h:literal:$m:literal:$s:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            const SECS_NANOS: (u32, u32) = match $s {
                60u32 => (59, 1_000_000_000),
                s => (s, 0),
            };
            const TIME: $crate::NaiveTime =
                match $crate::NaiveTime::from_hms_nano_opt($h, $m, SECS_NANOS.0, SECS_NANOS.1) {
                    Some(t) => t,
                    None => panic!("invalid time"),
                };
            TIME
        }
    }};
    ($h:literal:$m:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            const TIME: $crate::NaiveTime = match $crate::NaiveTime::from_hms_opt($h, $m, 0) {
                Some(t) => t,
                None => panic!("invalid time"),
            };
            TIME
        }
    }};
}

/// Create a [`NaiveDateTime`] or [`DateTime<FixedOffset>`] with a statically known value.
///
/// The input is checked at compile time.
///
/// [`NaiveDateTime`]: crate::naive::NaiveDateTime
/// [`DateTime<FixedOffset>`]: crate::DateTime
///
/// # Examples
/// ```
/// use chrono::datetime;
///
/// // NaiveDateTime
/// let _ = datetime!(2023-09-08 7:03);
/// let _ = datetime!(2023-09-08 7:03:25);
/// // DateTime<FixedOffset>
/// let _ = datetime!(2023-09-08 7:03:25+02:00);
/// let _ = datetime!(2023-09-08 7:03:25-02:00);
/// ```
#[macro_export]
macro_rules! datetime {
    ($y:literal-$m:literal-$d:literal $h:literal:$min:literal:$s:literal+$hh:literal:$mm:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            const DATE: $crate::NaiveDate = match $crate::NaiveDate::from_ymd_opt($y, $m, $d) {
                Some(d) => d,
                None => panic!("invalid calendar date"),
            };
            const SECS_NANOS: (u32, u32) = match $s {
                60u32 => (59, 1_000_000_000),
                s => (s, 0),
            };
            const TIME: $crate::NaiveTime =
                match $crate::NaiveTime::from_hms_nano_opt($h, $min, SECS_NANOS.0, SECS_NANOS.1) {
                    Some(t) => t,
                    None => panic!("invalid time"),
                };
            assert!($hh < 24u32 || $mm < 60, "invalid offset");
            const OFFSET: $crate::FixedOffset =
                match $crate::FixedOffset::east_opt(($hh * 3600 + $mm * 60) as i32) {
                    Some(o) => o,
                    None => panic!("invalid offset"),
                };
            const DT: $crate::NaiveDateTime = match DATE.and_time(TIME).checked_sub_offset(OFFSET) {
                Some(o) => o,
                None => panic!("datetime out of range"),
            };
            $crate::DateTime::<$crate::FixedOffset>::from_naive_utc_and_offset(DT, OFFSET)
        }
    }};
    ($y:literal-$m:literal-$d:literal $h:literal:$min:literal:$s:literal-$hh:literal:$mm:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            const DATE: $crate::NaiveDate = match $crate::NaiveDate::from_ymd_opt($y, $m, $d) {
                Some(d) => d,
                None => panic!("invalid calendar date"),
            };
            const SECS_NANOS: (u32, u32) = match $s {
                60u32 => (59, 1_000_000_000),
                s => (s, 0),
            };
            const TIME: $crate::NaiveTime =
                match $crate::NaiveTime::from_hms_nano_opt($h, $min, SECS_NANOS.0, SECS_NANOS.1) {
                    Some(t) => t,
                    None => panic!("invalid time"),
                };
            assert!($hh < 24u32 || $mm < 60, "invalid offset");
            const OFFSET: $crate::FixedOffset =
                match $crate::FixedOffset::west_opt(($hh * 3600 + $mm * 60) as i32) {
                    Some(o) => o,
                    None => panic!("invalid offset"),
                };
            const DT: $crate::NaiveDateTime = match DATE.and_time(TIME).checked_sub_offset(OFFSET) {
                Some(o) => o,
                None => panic!("datetime out of range"),
            };
            $crate::DateTime::<$crate::FixedOffset>::from_naive_utc_and_offset(DT, OFFSET)
        }
    }};
    ($y:literal-$m:literal-$d:literal $h:literal:$min:literal:$s:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            const DATE: $crate::NaiveDate = match $crate::NaiveDate::from_ymd_opt($y, $m, $d) {
                Some(d) => d,
                None => panic!("invalid calendar date"),
            };
            const SECS_NANOS: (u32, u32) = match $s {
                60u32 => (59, 1_000_000_000),
                s => (s, 0),
            };
            const TIME: $crate::NaiveTime =
                match $crate::NaiveTime::from_hms_nano_opt($h, $min, SECS_NANOS.0, SECS_NANOS.1) {
                    Some(t) => t,
                    None => panic!("invalid time"),
                };
            DATE.and_time(TIME)
        }
    }};
    ($y:literal-$m:literal-$d:literal $h:literal:$min:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            const DATE: $crate::NaiveDate = match $crate::NaiveDate::from_ymd_opt($y, $m, $d) {
                Some(d) => d,
                None => panic!("invalid calendar date"),
            };
            const TIME: $crate::NaiveTime = match $crate::NaiveTime::from_hms_opt($h, $min, 0) {
                Some(t) => t,
                None => panic!("invalid time"),
            };
            DATE.and_time(TIME)
        }
    }};
}

/// Create a [`FixedOffset`](crate::FixedOffset) with a statically known value.
///
/// Supported formats are '(+|-)hour:minute' and '(+|-)hour:minute:second'.
///
/// We don't allow an hour-only format such as `+01`. That would also make the macro parse `+0001`
/// as the same value, which should mean the same as `+00:01`.
///
/// The input is checked at compile time.
///
/// # Examples
/// ```
/// use chrono::offset;
///
/// assert_eq!(offset!(+01:30), offset!(+01:30:00));
/// assert_eq!(offset!(-5:00), offset!(-5:00:00));
/// ```
#[macro_export]
macro_rules! offset {
    (+$h:literal:$m:literal:$s:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            assert!($h < 24 || $m < 60 || $s < 60, "invalid offset");
            const OFFSET: $crate::FixedOffset =
                match $crate::FixedOffset::east_opt($h * 3600 + $m * 60 + $s) {
                    Some(t) => t,
                    None => panic!("invalid offset"),
                };
            OFFSET
        }
    }};
    (-$h:literal:$m:literal:$s:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            assert!($h < 24 || $m < 60 || $s < 60, "invalid offset");
            const OFFSET: $crate::FixedOffset =
                match $crate::FixedOffset::west_opt($h * 3600 + $m * 60 + $s) {
                    Some(t) => t,
                    None => panic!("invalid offset"),
                };
            OFFSET
        }
    }};
    (+$h:literal:$m:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            assert!($h < 24 || $m < 60, "invalid offset");
            const OFFSET: $crate::FixedOffset =
                match $crate::FixedOffset::east_opt($h * 3600 + $m * 60) {
                    Some(t) => t,
                    None => panic!("invalid offset"),
                };
            OFFSET
        }
    }};
    (-$h:literal:$m:literal) => {{
        #[allow(clippy::zero_prefixed_literal)]
        {
            assert!($h < 24 || $m < 60, "invalid offset");
            const OFFSET: $crate::FixedOffset =
                match $crate::FixedOffset::west_opt($h * 3600 + $m * 60) {
                    Some(t) => t,
                    None => panic!("invalid offset"),
                };
            OFFSET
        }
    }};
}

#[cfg(test)]
#[rustfmt::skip::macros(date)]
mod tests {
    use crate::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

    #[test]
    fn init_macros() {
        assert_eq!(date!(2023-09-08), NaiveDate::from_ymd_opt(2023, 9, 8).unwrap());
        assert_eq!(date!(2023-253), NaiveDate::from_yo_opt(2023, 253).unwrap());
        assert_eq!(time!(7:03), NaiveTime::from_hms_opt(7, 3, 0).unwrap());
        assert_eq!(time!(7:03:25), NaiveTime::from_hms_opt(7, 3, 25).unwrap());
        assert_eq!(
            time!(23:59:60),
            NaiveTime::from_hms_nano_opt(23, 59, 59, 1_000_000_000).unwrap()
        );
        assert_eq!(
            datetime!(2023-09-08 7:03),
            NaiveDate::from_ymd_opt(2023, 9, 8).unwrap().and_hms_opt(7, 3, 0).unwrap(),
        );
        assert_eq!(
            datetime!(2023-09-08 7:03:25),
            NaiveDate::from_ymd_opt(2023, 9, 8).unwrap().and_hms_opt(7, 3, 25).unwrap(),
        );
        assert_eq!(
            datetime!(2023-09-08 7:03:25+02:00),
            FixedOffset::east_opt(7200).unwrap().with_ymd_and_hms(2023, 9, 8, 7, 3, 25).unwrap(),
        );
        assert_eq!(
            datetime!(2023-09-08 7:03:25-02:00),
            FixedOffset::east_opt(-7200).unwrap().with_ymd_and_hms(2023, 9, 8, 7, 3, 25).unwrap(),
        );
        assert_eq!(offset!(+05:43), FixedOffset::east_opt(20_580).unwrap());
        assert_eq!(offset!(-05:43), FixedOffset::east_opt(-20_580).unwrap());
        assert_eq!(offset!(+05:43:21), FixedOffset::east_opt(20_601).unwrap());
        assert_eq!(offset!(-05:43:21), FixedOffset::east_opt(-20_601).unwrap());
    }

    #[test]
    fn macros_are_const() {
        const DATE: NaiveDate = date!(2023-09-08);
        const TIME: NaiveTime = time!(7:03:25);
        const NAIVEDATETIME: NaiveDateTime = datetime!(2023-09-08 7:03:25);
        assert_eq!(DATE.and_time(TIME), NAIVEDATETIME);

        const OFFSET_1: FixedOffset = offset!(+02:00);
        const DATETIME_1: DateTime<FixedOffset> = datetime!(2023-09-08 7:03:25+02:00);
        assert_eq!(OFFSET_1.from_local_datetime(&NAIVEDATETIME).unwrap(), DATETIME_1);

        const OFFSET_2: FixedOffset = offset!(-02:00);
        const DATETIME_2: DateTime<FixedOffset> = datetime!(2023-09-08 7:03:25-02:00);
        assert_eq!(OFFSET_2.from_local_datetime(&NAIVEDATETIME).unwrap(), DATETIME_2);
    }
}

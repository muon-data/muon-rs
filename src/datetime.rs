// datetime.rs
//
// Copyright (c) 2019-2020  Douglas Lau
//
//! Module for RFC 3339 dates and times.
use crate::error::ParseError;
use serde::{de, ser};
use std::fmt;
use std::str::FromStr;

/// Date and time with offset
///
/// Formatted and validated as
/// [RFC 3339](https://tools.ietf.org/html/rfc3339#section-5.6) `date-time`.
/// ```
/// use muon_rs::DateTime;
/// let datetime = "2019-08-07T16:35:21.363-06:00".parse::<DateTime>().unwrap();
/// let date = datetime.date();
/// let time = datetime.time();
/// let offset = datetime.time_offset();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DateTime {
    date: Date,
    time: Time,
    time_offset: TimeOffset,
}

/// Date with no time or offset
///
/// Formatted and validated as
/// [RFC 3339](https://tools.ietf.org/html/rfc3339#section-5.6) `full-date`.
/// ```
/// use muon_rs::Date;
/// let date = "2019-08-07".parse::<Date>().unwrap();
/// let year = date.year();
/// let month = date.month();
/// let day = date.day();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}

/// Time with no date or offset
///
/// Formatted and validated as
/// [RFC 3339](https://tools.ietf.org/html/rfc3339#section-5.6) `partial-time`.
/// ```
/// use muon_rs::Time;
/// let time = "16:35:21.363".parse::<Time>().unwrap();
/// let hour = time.hour();
/// let minute = time.minute();
/// let second = time.second();
/// let nanosecond = time.nanosecond();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Time {
    hour: u8,
    minute: u8,
    second: u8,
    nanosecond: u32,
}

/// Fixed time offset
///
/// Formatted and validated as
/// [RFC 3339](https://tools.ietf.org/html/rfc3339#section-5.6) `time-offset`.
/// ```
/// use muon_rs::TimeOffset;
/// let offset = "-05:00".parse::<TimeOffset>().unwrap();
/// let seconds = offset.seconds();
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeOffset(_TimeOffset);

/// Private time offset
#[derive(Clone, Copy, Debug, PartialEq)]
enum _TimeOffset {
    Z,
    Positive(u8, u8),
    Negative(u8, u8),
}

/// Determine the number of days in a month
fn days_in_month(year: u16, month: u8) -> Option<u8> {
    match month {
        // April, June, Septemper, November
        4 | 6 | 9 | 11 => Some(30),
        // January, March, May, July, August, October, December
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        // February
        2 => Some(if is_leap_year(year) { 29 } else { 28 }),
        // Not a real month
        _ => None,
    }
}

/// Check if a year is a leap year
fn is_leap_year(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Convert ASCII digit to a number
fn digit(b: u8) -> Option<u8> {
    if b >= b'0' && b <= b'9' {
        Some(b - b'0')
    } else {
        None
    }
}

/// Parse a 4-digit ASCII decimal number
fn parse_4_digits(ascii: &[u8]) -> Option<u16> {
    if ascii.len() == 4 {
        if let (Some(b0), Some(b1), Some(b2), Some(b3)) = (
            digit(ascii[0]),
            digit(ascii[1]),
            digit(ascii[2]),
            digit(ascii[3]),
        ) {
            return Some(
                u16::from(b0) * 1000
                    + u16::from(b1) * 100
                    + u16::from(b2) * 10
                    + u16::from(b3),
            );
        }
    }
    None
}

/// Parse a 4-digit year
fn parse_year(year: &[u8]) -> Option<u16> {
    match parse_4_digits(year) {
        Some(year) => Some(year),
        _ => None,
    }
}

/// Parse a 2-digit ASCII decimal number
fn parse_2_digits(ascii: &[u8]) -> Option<u8> {
    if ascii.len() == 2 {
        if let (Some(b0), Some(b1)) = (digit(ascii[0]), digit(ascii[1])) {
            return Some(b0 * 10 + b1);
        }
    }
    None
}

/// Parse a 2-digit month
fn parse_month(month: &[u8]) -> Option<u8> {
    match parse_2_digits(month) {
        Some(month) if month >= 1 && month <= 12 => Some(month),
        _ => None,
    }
}

/// Parse a 2-digit day
fn parse_day(day: &[u8]) -> Option<u8> {
    match parse_2_digits(day) {
        Some(day) if day >= 1 && day <= 31 => Some(day),
        _ => None,
    }
}

/// Parse a 2-digit hour
fn parse_hour(hour: &[u8]) -> Option<u8> {
    match parse_2_digits(hour) {
        Some(hour) if hour < 24 => Some(hour),
        _ => None,
    }
}

/// Parse a 2-digit minute
fn parse_minute(minute: &[u8]) -> Option<u8> {
    match parse_2_digits(minute) {
        Some(minute) if minute < 60 => Some(minute),
        _ => None,
    }
}

/// Parse a 2-digit second
fn parse_second(second: &[u8], leap_sec: bool) -> Option<u8> {
    let max_seconds = if leap_sec { 60 } else { 59 };
    match parse_2_digits(second) {
        Some(second) if second <= max_seconds => Some(second),
        _ => None,
    }
}

/// Parse a nanosecond
fn parse_nanosecond(nano: &[u8]) -> Option<u32> {
    if nano.is_empty() {
        Some(0)
    } else if nano.len() >= 2 && nano[0] == b'.' {
        let mut ns = 0;
        for (i, b) in nano[1..].iter().enumerate() {
            match digit(*b) {
                Some(b) => {
                    if i < 9 {
                        ns += u32::from(b) * 10_u32.pow(8 - i as u32);
                    }
                }
                None => return None,
            }
        }
        Some(ns)
    } else {
        None
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.date.fmt(f)?;
        write!(f, "T")?;
        self.time.fmt(f)?;
        self.time_offset.fmt(f)
    }
}

impl FromStr for DateTime {
    type Err = ParseError;

    fn from_str(datetime: &str) -> Result<Self, Self::Err> {
        DateTime::new(datetime.as_bytes())
    }
}

impl ser::Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> de::Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct DateTimeVisitor;

        impl<'de> de::Visitor<'de> for DateTimeVisitor {
            type Value = DateTime;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "DateTime")
            }

            fn visit_str<E: de::Error>(
                self,
                s: &str,
            ) -> Result<Self::Value, E> {
                match s.parse() {
                    Ok(datetime) => Ok(datetime),
                    Err(_) => Err(de::Error::invalid_value(
                        de::Unexpected::Str(&s),
                        &self,
                    )),
                }
            }
        }
        deserializer.deserialize_str(DateTimeVisitor)
    }
}

impl DateTime {
    /// Create a new datetime
    fn new(bytes: &[u8]) -> Result<DateTime, ParseError> {
        // FIXME: check for leap seconds
        // date (10 bytes) + "T" + time (8+ bytes) + offset (1 or 6 bytes)
        let len = bytes.len();
        if len >= 20 {
            let offset = TimeOffset::rindex(bytes);
            if offset >= 11 && bytes[10] == b'T' {
                let date = Date::new(&bytes[..10])?;
                let time = Time::new(&bytes[11..offset], false)?;
                let time_offset = TimeOffset::new(&bytes[offset..])?;
                return Ok(DateTime {
                    date,
                    time,
                    time_offset,
                });
            }
        }
        Err(ParseError::ExpectedDateTime)
    }
    /// Get the date
    pub fn date(&self) -> Date {
        self.date
    }
    /// Get the time
    pub fn time(&self) -> Time {
        self.time
    }
    /// Get the time offset
    pub fn time_offset(&self) -> TimeOffset {
        self.time_offset
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for Date {
    type Err = ParseError;

    fn from_str(date: &str) -> Result<Self, Self::Err> {
        Date::new(date.as_bytes())
    }
}

impl ser::Serialize for Date {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Date {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct DateVisitor;

        impl<'de> de::Visitor<'de> for DateVisitor {
            type Value = Date;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "Date")
            }

            fn visit_str<E: de::Error>(
                self,
                s: &str,
            ) -> Result<Self::Value, E> {
                match s.parse() {
                    Ok(date) => Ok(date),
                    Err(_) => Err(de::Error::invalid_value(
                        de::Unexpected::Str(&s),
                        &self,
                    )),
                }
            }
        }
        deserializer.deserialize_str(DateVisitor)
    }
}

impl Date {
    /// Create a new date
    fn new(bytes: &[u8]) -> Result<Self, ParseError> {
        if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
            if let Some(year) = parse_year(&bytes[..4]) {
                if let Some(month) = parse_month(&bytes[5..7]) {
                    if let Some(mdays) = days_in_month(year, month) {
                        if let Some(day) = parse_day(&bytes[8..]) {
                            if day <= mdays {
                                return Ok(Date { year, month, day });
                            }
                        }
                    }
                }
            }
        }
        Err(ParseError::ExpectedDate)
    }
    /// Get the year
    pub fn year(&self) -> u16 {
        self.year
    }
    /// Get the month (1-12)
    pub fn month(&self) -> u8 {
        self.month
    }
    /// Get the day of month (1-31)
    pub fn day(&self) -> u8 {
        self.day
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)?;
        if self.nanosecond > 0 {
            let ns = format!("{:09}", self.nanosecond);
            write!(f, ".{}", ns.trim_end_matches('0'))
        } else {
            Ok(())
        }
    }
}

impl FromStr for Time {
    type Err = ParseError;

    fn from_str(time: &str) -> Result<Self, Self::Err> {
        Time::new(time.as_bytes(), false)
    }
}

impl ser::Serialize for Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> de::Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct TimeVisitor;

        impl<'de> de::Visitor<'de> for TimeVisitor {
            type Value = Time;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "Time")
            }

            fn visit_str<E: de::Error>(
                self,
                s: &str,
            ) -> Result<Self::Value, E> {
                match s.parse::<Time>() {
                    Ok(time) => Ok(time),
                    Err(_) => Err(de::Error::invalid_value(
                        de::Unexpected::Str(&s),
                        &self,
                    )),
                }
            }
        }
        deserializer.deserialize_str(TimeVisitor)
    }
}

impl Time {
    /// Create a new time
    fn new(bytes: &[u8], leap_sec: bool) -> Result<Self, ParseError> {
        if bytes.len() >= 8 && bytes[2] == b':' && bytes[5] == b':' {
            if let Some(hour) = parse_hour(&bytes[..2]) {
                if let Some(minute) = parse_minute(&bytes[3..5]) {
                    if let Some(second) = parse_second(&bytes[6..8], leap_sec) {
                        if let Some(nanosecond) = parse_nanosecond(&bytes[8..])
                        {
                            return Ok(Time {
                                hour,
                                minute,
                                second,
                                nanosecond,
                            });
                        }
                    }
                }
            }
        }
        Err(ParseError::ExpectedTime)
    }
    /// Get the hour (0-23)
    pub fn hour(&self) -> u8 {
        self.hour
    }
    /// Get the minute (0-59)
    pub fn minute(&self) -> u8 {
        self.minute
    }
    /// Get the second (0-59, or 60 for leap second)
    pub fn second(&self) -> u8 {
        self.second
    }
    /// Get the nanosecond (0-999_999_999)
    pub fn nanosecond(&self) -> u32 {
        self.nanosecond
    }
}

impl fmt::Display for TimeOffset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            _TimeOffset::Z => write!(f, "Z"),
            _TimeOffset::Positive(h, m) => write!(f, "+{:02}:{:02}", h, m),
            _TimeOffset::Negative(h, m) => write!(f, "-{:02}:{:02}", h, m),
        }
    }
}

impl FromStr for TimeOffset {
    type Err = ParseError;

    fn from_str(offset: &str) -> Result<Self, Self::Err> {
        TimeOffset::new(offset.as_bytes())
    }
}

impl TimeOffset {
    /// Create a new time offset
    fn new(bytes: &[u8]) -> Result<Self, ParseError> {
        if bytes.len() == 1 && bytes[0] == b'Z' {
            return Ok(TimeOffset(_TimeOffset::Z));
        } else if bytes.len() == 6
            && bytes[3] == b':'
            && (bytes[0] == b'+' || bytes[0] == b'-')
        {
            if let Some(h) = parse_hour(&bytes[1..3]) {
                if let Some(m) = parse_minute(&bytes[4..6]) {
                    if bytes[0] == b'+' {
                        return Ok(TimeOffset(_TimeOffset::Positive(h, m)));
                    } else {
                        return Ok(TimeOffset(_TimeOffset::Negative(h, m)));
                    }
                }
            }
        }
        Err(ParseError::ExpectedTimeOffset)
    }
    /// Find possible index of a TimeOffset at the end of a byte slice
    fn rindex(bytes: &[u8]) -> usize {
        const MAX: usize = std::usize::MAX;
        let len = bytes.len();
        match len {
            1..=MAX if bytes[len - 1] == b'Z' => len - 1,
            6..=MAX => len - 6,
            _ => 0,
        }
    }
    /// Get the time offset in seconds
    pub fn seconds(&self) -> i32 {
        match self.0 {
            _TimeOffset::Z => 0,
            _TimeOffset::Positive(h, m) => hour_minute_to_seconds(h, m),
            _TimeOffset::Negative(h, m) => -hour_minute_to_seconds(h, m),
        }
    }
}

/// Calculate seconds from hour and minute
fn hour_minute_to_seconds(hour: u8, minute: u8) -> i32 {
    3600 * i32::from(hour) + 60 * i32::from(minute)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn date_time_ok() -> Result<(), Box<ParseError>> {
        assert_eq!(
            2011,
            "2011-01-01T12:30:15Z".parse::<DateTime>()?.date().year()
        );
        assert_eq!(
            4,
            "2002-04-02T04:57:19.001+00:00"
                .parse::<DateTime>()?
                .date()
                .month()
        );
        assert_eq!(
            15,
            "1975-03-15T19:23:00+07:00"
                .parse::<DateTime>()?
                .date()
                .day()
        );
        assert_eq!(
            22,
            "2009-10-03T22:03:19-05:00"
                .parse::<DateTime>()?
                .time()
                .hour()
        );
        assert_eq!(
            59,
            "2025-09-29T14:59:13.392853953+10:45"
                .parse::<DateTime>()?
                .time()
                .minute()
        );
        assert_eq!(
            48,
            "2015-05-27T18:31:48.123-06:00"
                .parse::<DateTime>()?
                .time()
                .second()
        );
        assert_eq!(
            987_654_321,
            "2003-08-22T01:55:11.987654321+02:30"
                .parse::<DateTime>()?
                .time()
                .nanosecond()
        );
        assert_eq!(
            0,
            "2007-01-11T05:45:12+04:00"
                .parse::<DateTime>()?
                .time()
                .nanosecond()
        );
        assert_eq!(
            -21600,
            "2012-06-21T19:03:00.0-06:00"
                .parse::<DateTime>()?
                .time_offset()
                .seconds()
        );
        Ok(())
    }

    #[test]
    fn date_time_err() -> Result<(), Box<ParseError>> {
        assert!("".parse::<DateTime>().is_err());
        assert!("0000".parse::<DateTime>().is_err());
        assert!("0000-00-00T00:00:00Z".parse::<DateTime>().is_err());
        assert!("2000-01-01t00:00:00Z".parse::<DateTime>().is_err());
        assert!("2000-01-01TT00:00:00Z".parse::<DateTime>().is_err());
        assert!("2000-01-01 00:00:00Z".parse::<DateTime>().is_err());
        assert!("2000-01-01T00:00:00 Z".parse::<DateTime>().is_err());
        assert!("2000-01-01T00:00:00=00:00".parse::<DateTime>().is_err());
        assert!("2000-01-01T00:00:00.00 +00:00".parse::<DateTime>().is_err());
        assert!("2000-01-01T00:00:00.00.-00:00".parse::<DateTime>().is_err());
        Ok(())
    }

    #[test]
    fn date_ok() -> Result<(), Box<ParseError>> {
        assert_eq!(2011, "2011-01-01".parse::<Date>()?.year());
        assert_eq!(2050, "2050-04-30".parse::<Date>()?.year());
        assert_eq!(1, "1999-01-31".parse::<Date>()?.month());
        assert_eq!(12, "2004-12-01".parse::<Date>()?.month());
        assert_eq!(1, "1950-09-01".parse::<Date>()?.day());
        assert_eq!(31, "2019-07-31".parse::<Date>()?.day());
        assert_eq!(29, "2400-02-29".parse::<Date>()?.day());
        assert_eq!(29, "2004-02-29".parse::<Date>()?.day());
        assert_eq!(29, "2000-02-29".parse::<Date>()?.day());
        Ok(())
    }

    #[test]
    fn date_err() -> Result<(), Box<ParseError>> {
        assert!("".parse::<Date>().is_err());
        assert!("0000".parse::<Date>().is_err());
        assert!("0000-00".parse::<Date>().is_err());
        assert!("0000-00-".parse::<Date>().is_err());
        assert!("0000-00-0".parse::<Date>().is_err());
        assert!("0000-00-00".parse::<Date>().is_err());
        assert!("1999-00-01".parse::<Date>().is_err());
        assert!("2010-01-32".parse::<Date>().is_err());
        assert!("2011-04-31".parse::<Date>().is_err());
        assert!("2015-13-01".parse::<Date>().is_err());
        assert!("2018-01-00".parse::<Date>().is_err());
        assert!("1900-02-29".parse::<Date>().is_err());
        assert!("2019:07-31".parse::<Date>().is_err());
        Ok(())
    }

    #[test]
    fn time_ok() -> Result<(), Box<ParseError>> {
        assert_eq!(0, "00:00:00".parse::<Time>()?.hour());
        assert_eq!(23, "23:00:00".parse::<Time>()?.hour());
        assert_eq!(0, "12:00:34".parse::<Time>()?.minute());
        assert_eq!(45, "12:45:34".parse::<Time>()?.minute());
        assert_eq!(0, "12:34:00".parse::<Time>()?.second());
        assert_eq!(15, "12:45:15".parse::<Time>()?.second());
        Ok(())
    }

    #[test]
    fn time_err() -> Result<(), Box<ParseError>> {
        assert!("".parse::<Time>().is_err());
        assert!("00".parse::<Time>().is_err());
        assert!("00:00".parse::<Time>().is_err());
        assert!("00:00:".parse::<Time>().is_err());
        assert!("00:00:0".parse::<Time>().is_err());
        assert!("00;00:00".parse::<Time>().is_err());
        assert!("00:00:00:0".parse::<Time>().is_err());
        assert!("24:00:00".parse::<Time>().is_err());
        assert!("00:60:00".parse::<Time>().is_err());
        assert!("00:00:60".parse::<Time>().is_err());
        Ok(())
    }

    #[test]
    fn offset_ok() -> Result<(), Box<ParseError>> {
        assert_eq!(0, "Z".parse::<TimeOffset>()?.seconds());
        assert_eq!(0, "-00:00".parse::<TimeOffset>()?.seconds());
        assert_eq!(3600, "+01:00".parse::<TimeOffset>()?.seconds());
        assert_eq!(-18000, "-05:00".parse::<TimeOffset>()?.seconds());
        assert_eq!(-1800, "-00:30".parse::<TimeOffset>()?.seconds());
        assert_eq!(38700, "+10:45".parse::<TimeOffset>()?.seconds());
        assert_eq!(86340, "+23:59".parse::<TimeOffset>()?.seconds());
        Ok(())
    }

    #[test]
    fn offset_err() -> Result<(), Box<ParseError>> {
        assert!("".parse::<TimeOffset>().is_err());
        assert!("00:00".parse::<TimeOffset>().is_err());
        assert!("0000".parse::<TimeOffset>().is_err());
        assert!("_00;00".parse::<TimeOffset>().is_err());
        assert!(" 00:00".parse::<TimeOffset>().is_err());
        assert!("+0A:00".parse::<TimeOffset>().is_err());
        assert!("+00:60".parse::<TimeOffset>().is_err());
        assert!("+24:00".parse::<TimeOffset>().is_err());
        Ok(())
    }
}

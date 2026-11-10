//! A day of the calendar, read from a tool's `YYYY-MM-DD` text.

use serde::{Deserialize, Deserializer, Serialize};

/// A day of the Gregorian calendar, such as 3 November 2026. It serializes
/// as its year, month and day, so the page never parses text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Day {
    year: u16,
    month: u8,
    day: u8,
}

impl Day {
    /// Returns the day that `text` writes as `YYYY-MM-DD`, or `None` when
    /// `text` is not that form or names no day of the calendar, such as
    /// `2026-13-01` or `2026-02-30`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('-');
        let year = number(parts.next()?, 4)?;
        let month = number(parts.next()?, 2)?;
        let day = number(parts.next()?, 2)?;
        if parts.next().is_some() {
            return None;
        }
        let year = u16::try_from(year).ok()?;
        let month = u8::try_from(month).ok()?;
        let day = u8::try_from(day).ok()?;
        (day >= 1 && day <= days_in(year, month)?).then_some(Self { year, month, day })
    }

    /// Returns the year, such as 2026.
    #[must_use]
    pub const fn year(&self) -> u16 {
        self.year
    }

    /// Returns the month, from 1 for January to 12 for December.
    #[must_use]
    pub const fn month(&self) -> u8 {
        self.month
    }

    /// Returns the day of the month, from 1.
    #[must_use]
    pub const fn day(&self) -> u8 {
        self.day
    }
}

/// Returns the number that `digits` writes with exactly `width` ASCII
/// digits.
fn number(digits: &str, width: usize) -> Option<u32> {
    (digits.len() == width && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

/// Returns the days in `month` of `year`, or `None` for a month that is not
/// from 1 to 12.
const fn days_in(year: u16, month: u8) -> Option<u8> {
    let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if leap => Some(29),
        2 => Some(28),
        _ => None,
    }
}

/// Reads a day that a tool may print as `YYYY-MM-DD`, or leave out, as
/// `None` when it is missing, `null` or not a day.
pub(super) fn day_or_none<'de, D>(deserializer: D) -> Result<Option<Day>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .and_then(Day::parse))
}

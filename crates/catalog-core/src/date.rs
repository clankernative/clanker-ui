//! Gregorian ISO date primitives shared by the static date component contracts.
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Date {
    year: u16,
    month: u8,
    day: u8,
}
impl Date {
    pub const fn year(self) -> u16 {
        self.year
    }
    pub const fn month(self) -> u8 {
        self.month
    }
    pub const fn day(self) -> u8 {
        self.day
    }

    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, String> {
        if !(1..=9999).contains(&year)
            || !(1..=12).contains(&month)
            || !(1..=Self::days_in_month(year, month)).contains(&day)
        {
            return Err("date must be a valid Gregorian date in years 0001..=9999".into());
        }
        Ok(Self { year, month, day })
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.len() != 10
            || s.as_bytes()[4] != b'-'
            || s.as_bytes()[7] != b'-'
            || !s
                .bytes()
                .enumerate()
                .all(|(i, b)| matches!(i, 4 | 7) || b.is_ascii_digit())
        {
            return Err("date must use YYYY-MM-DD".into());
        }
        Self::new(
            s[..4].parse().map_err(|_| "invalid year")?,
            s[5..7].parse().map_err(|_| "invalid month")?,
            s[8..10].parse().map_err(|_| "invalid day")?,
        )
    }
    pub const fn leap(year: u16) -> bool {
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
    }
    pub const fn days_in_month(year: u16, month: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::leap(year) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    }
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
    /// Sunday=0, matching the explicit browser contract.
    pub fn weekday(self) -> u8 {
        let mut y = self.year as i64;
        let m = self.month as i64;
        let d = self.day as i64;
        if m < 3 {
            y -= 1;
        }
        ((y + y / 4 - y / 100
            + y / 400
            + [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4][(m - 1) as usize]
            + d)
            % 7) as u8
    }
    pub fn add_days(self, delta: i32) -> Option<Self> {
        let mut date = self;
        let n = delta.unsigned_abs();
        for _ in 0..n {
            if delta >= 0 {
                if date.day < Self::days_in_month(date.year, date.month) {
                    date.day += 1
                } else if date.month < 12 {
                    date.month += 1;
                    date.day = 1
                } else if date.year < 9999 {
                    date.year += 1;
                    date.month = 1;
                    date.day = 1
                } else {
                    return None;
                }
            } else if date.day > 1 {
                date.day -= 1
            } else if date.month > 1 {
                date.month -= 1;
                date.day = Self::days_in_month(date.year, date.month)
            } else if date.year > 1 {
                date.year -= 1;
                date.month = 12;
                date.day = 31
            } else {
                return None;
            }
        }
        Some(date)
    }
    pub fn month_start(self) -> Self {
        Self { day: 1, ..self }
    }
    pub fn add_months(self, delta: i32) -> Option<Self> {
        let index = i64::from(self.year) * 12 + i64::from(self.month) - 1 + i64::from(delta);
        if !(12..=119_999).contains(&index) {
            return None;
        }
        let year = (index / 12) as u16;
        let month = (index % 12 + 1) as u8;
        Some(Self {
            year,
            month,
            day: self.day.min(Self::days_in_month(year, month)),
        })
    }
}
impl TryFrom<String> for Date {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        Self::parse(&s)
    }
}
impl From<Date> for String {
    fn from(d: Date) -> String {
        d.iso()
    }
}
impl Display for Date {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_gregorian_iso_and_edges() {
        assert!(Date::parse("2000-02-29").is_ok());
        for s in [
            "1900-02-29",
            "0000-01-01",
            "10000-01-01",
            "2024-04-31",
            "2024-1-01",
        ] {
            assert!(Date::parse(s).is_err(), "{s}");
        }
        assert_eq!(Date::parse("0001-01-01").unwrap().weekday(), 1);
        assert_eq!(Date::parse("9999-12-31").unwrap().add_days(1), None);
        assert_eq!(
            Date::parse("2024-01-31")
                .unwrap()
                .add_months(1)
                .unwrap()
                .iso(),
            "2024-02-29"
        );
    }
}

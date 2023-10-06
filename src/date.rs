use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// Simple representation of calendar date, parsed and represented as YYYY-MM-DD
pub struct Date {
    /// Year, usually 1970+
    pub year: u16,
    /// Month number, 1-12
    pub month: u8,
    /// Month's day, 1-31
    pub day: u8,
}

impl Display for Date {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        ))
    }
}

/// Possible errors which may occur when parsing date from string
#[derive(Debug, thiserror::Error)]
pub enum DateParseError {
    #[error("Invalid number of date components")]
    InvalidComponents,
    #[error("Error parsing date's year component")]
    YearParseError,
    #[error("Error parsing date's month component")]
    MonthParseError,
    #[error("Error parsing date's day component")]
    DayParseError,
}

impl FromStr for Date {
    type Err = DateParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('-');
        let year = parts.next().ok_or(Self::Err::InvalidComponents)?;
        let month = parts.next().ok_or(Self::Err::InvalidComponents)?;
        let day = parts.next().ok_or(Self::Err::InvalidComponents)?;

        if parts.next().is_some() {
            return Err(Self::Err::InvalidComponents);
        }

        Ok(Self {
            year: year.parse().map_err(|_| Self::Err::YearParseError)?,
            month: month.parse().map_err(|_| Self::Err::MonthParseError)?,
            day: day.parse().map_err(|_| Self::Err::DayParseError)?,
        })
    }
}

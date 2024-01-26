use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

/// Release dates are in ISO 8601 date format (YYYY-MM-DD)
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ReleaseDate(String);

impl ReleaseDate {
    /// Creates a [`ReleaseDate`] instance for the current date.
    #[must_use]
    pub fn today() -> Self {
        chrono::Utc::now().into()
    }
}

/// An error for release dates that cannot be parsed.
#[derive(Debug, Error)]
#[error("Could not parse release date '{0}' as YYYY-MM-DD.\nReason: {1}")]
pub struct ParseReleaseDateError(String, String);

impl FromStr for ReleaseDate {
    type Err = ParseReleaseDateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        format!("{value}T00:00:00Z")
            .parse::<chrono::DateTime<chrono::Utc>>()
            .map_err(|e| ParseReleaseDateError(value.to_string(), e.to_string()))
            .map(|_| ReleaseDate(value.to_string()))
    }
}

impl Display for ReleaseDate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<chrono::DateTime<chrono::Utc>> for ReleaseDate {
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        value
            .format("%Y-%m-%d")
            .to_string()
            .parse()
            .expect("should be a valid release date")
    }
}

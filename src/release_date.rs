use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

/// Release dates are in ISO 8601 date format (YYYY-MM-DD)
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ReleaseDate(pub(crate) chrono::DateTime<chrono::Utc>);

impl ReleaseDate {
    /// Creates a [`ReleaseDate`] instance for the current date.
    #[must_use]
    pub fn today() -> Self {
        Self(chrono::Utc::now())
    }
}

/// An error for release dates that cannot be parsed.
#[derive(Debug, Error)]
#[error("Could not parse release date '{0}' as YYYY-MM-DD.\nError: {1}")]
pub struct ParseReleaseDateError(String, chrono::ParseError);

impl FromStr for ReleaseDate {
    type Err = ParseReleaseDateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        format!("{value}T00:00:00Z")
            .parse::<chrono::DateTime<chrono::Utc>>()
            .map_err(|e| ParseReleaseDateError(value.to_string(), e))
            .map(Self)
    }
}

impl Display for ReleaseDate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d"))
    }
}

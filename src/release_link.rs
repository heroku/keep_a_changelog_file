use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

/// URI to the set of changes in a release.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ReleaseLink(String);

/// Error for when a release link cannot be parsed.
#[derive(Debug, Error)]
#[error("Could not parse release link '{0}' as a URI.\nReason: {1}")]
pub struct ParseReleaseLinkError(String, String);

impl FromStr for ReleaseLink {
    type Err = ParseReleaseLinkError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        uriparse::URI::try_from(value)
            .map_err(|e| ParseReleaseLinkError(value.to_string(), e.to_string()))
            .map(|_| ReleaseLink(value.to_string()))
    }
}

impl Display for ReleaseLink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

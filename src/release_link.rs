use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

/// URI to the set of changes in a release.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ReleaseLink(uriparse::URI<'static>);

/// Error for when a release link cannot be parsed.
#[derive(Debug, Error)]
#[error("Could not parse release link '{0}' as a URI.\nError: {1}")]
pub struct ParseReleaseLinkError(String, uriparse::URIError);

impl FromStr for ReleaseLink {
    type Err = ParseReleaseLinkError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        uriparse::URI::try_from(value)
            .map(uriparse::URI::into_owned)
            .map_err(|e| ParseReleaseLinkError(value.to_string(), e))
            .map(Self)
    }
}

impl Display for ReleaseLink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

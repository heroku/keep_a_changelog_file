use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

/// The version of a release in [Semantic Versioning](https://semver.org/) format.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ReleaseVersion(String);

/// An error for when the version cannot be parsed into [Semantic Versioning](https://semver.org/) format.
#[derive(Debug, Error)]
#[error("Could not parse version '{0}' as semver.\nReason: {1}")]
pub struct ParseVersionError(String, String);

impl FromStr for ReleaseVersion {
    type Err = ParseVersionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse::<semver::Version>()
            .map_err(|e| ParseVersionError(value.to_string(), e.to_string()))
            .map(|_| ReleaseVersion(value.to_string()))
    }
}

impl Display for ReleaseVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

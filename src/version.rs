use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

/// The version of a release in [Semantic Versioning](https://semver.org/) format.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Version(semver::Version);

/// An error for when the version cannot be parsed into [Semantic Versioning](https://semver.org/) format.
#[derive(Debug, Error)]
#[error("Could not parse version '{0}' as semver.\nError: {1}")]
pub struct ParseVersionError(String, semver::Error);

impl FromStr for Version {
    type Err = ParseVersionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse::<semver::Version>()
            .map_err(|e| ParseVersionError(value.to_string(), e))
            .map(Self)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

/// A release tag can be used to indicate:
/// - If a release was yanked due to a serious bug or security issue.
/// - If a release version was bumped but there were no changes which can be common in projects that
///   use a fixed version strategy to release a set of artifacts.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ReleaseTag {
    /// A yanked release.
    Yanked,
    /// A release with no changes.
    NoChanges,
}

impl Display for ReleaseTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseTag::Yanked => write!(f, "YANKED"),
            ReleaseTag::NoChanges => write!(f, "NO CHANGES"),
        }
    }
}

#[derive(Debug, Error)]
#[error("Could not parse release tag '{0}'\nExpected: YANKED | NO CHANGES")]
pub struct ParseReleaseTagError(String);

impl FromStr for ReleaseTag {
    type Err = ParseReleaseTagError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "no changes" => Ok(ReleaseTag::NoChanges),
            "yanked" => Ok(ReleaseTag::Yanked),
            _ => Err(ParseReleaseTagError(value.to_string())),
        }
    }
}

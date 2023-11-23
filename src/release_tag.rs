use std::fmt::{Display, Formatter};

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

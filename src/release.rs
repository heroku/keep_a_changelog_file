use crate::changes::Changes;
use crate::release_date::ReleaseDate;
use crate::release_link::ReleaseLink;
use crate::release_tag::ReleaseTag;
use crate::release_version::ReleaseVersion;

/// Represents release information such as the version, date, link to release, list of changes, and so on.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Release {
    /// The version of the release in [semver](https://semver.org/spec/v2.0.0.html) format.
    pub version: ReleaseVersion,
    /// The date the release was created.
    pub date: ReleaseDate,
    /// A tag can be used to indicate if a release was yanked or when the version was bumped with no changes.
    pub tag: Option<ReleaseTag>,
    /// The link to the release.
    pub link: Option<ReleaseLink>,
    /// An ordered map of the changes in a release grouped by the type of change.
    pub changes: Changes,
}

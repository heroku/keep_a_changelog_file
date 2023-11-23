use crate::{Release, Version};
use indexmap::IndexMap;

/// The list of releases in the changelog.
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct Releases(pub(crate) IndexMap<Version, Release>);

impl Releases {
    /// Returns the release matching the requested `version` if it exists in the changelog.
    #[must_use]
    pub fn get_version(&self, version: &Version) -> Option<&Release> {
        self.0.get(version)
    }
}

impl<'a> IntoIterator for &'a Releases {
    type Item = &'a Release;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.values().collect::<Vec<_>>().into_iter()
    }
}

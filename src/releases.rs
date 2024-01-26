use crate::{Release, ReleaseVersion};
use indexmap::IndexMap;

/// The list of releases in the changelog.
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct Releases(IndexMap<ReleaseVersion, Release>);

impl Releases {
    pub(crate) fn from_iter<I: IntoIterator<Item = (ReleaseVersion, Release)>>(
        iterable: I,
    ) -> Releases {
        Self(IndexMap::from_iter(iterable))
    }
}

impl Releases {
    /// Returns the release matching the requested `version` if it exists in the changelog.
    #[must_use]
    pub fn get_version(&self, version: &ReleaseVersion) -> Option<&Release> {
        self.0.get(version)
    }

    /// Returns true if the requested `version` exists in the changelog.
    #[must_use]
    pub fn contains_version(&self, version: &ReleaseVersion) -> bool {
        self.0.contains_key(version)
    }

    /// Returns an iterator over the version/release pairs
    #[must_use]
    pub fn iter(&self) -> std::vec::IntoIter<(&ReleaseVersion, &Release)> {
        self.into_iter()
    }
}

impl IntoIterator for Releases {
    type Item = (ReleaseVersion, Release);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().collect::<Vec<_>>().into_iter()
    }
}

impl<'a> IntoIterator for &'a Releases {
    type Item = (&'a ReleaseVersion, &'a Release);

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().collect::<Vec<_>>().into_iter()
    }
}

use crate::ChangeGroup;
use indexmap::IndexMap;

/// Represents the changes that went into a release.
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct Changes(pub(crate) IndexMap<ChangeGroup, Vec<String>>);

impl Changes {
    /// Returns true if there are no changes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|(_, items)| items.is_empty())
    }
}

impl<'a> IntoIterator for &'a Changes {
    type Item = (&'a ChangeGroup, &'a Vec<String>);

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().collect::<Vec<_>>().into_iter()
    }
}

use crate::ChangeGroup;
use indexmap::IndexMap;

/// Represents the changes that went into a release.
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct Changes(IndexMap<ChangeGroup, Vec<String>>);

impl Changes {
    /// Returns true if there are no changes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|(_, items)| items.is_empty())
    }

    /// Returns an iterator over the change group/list of changes pairs
    #[must_use]
    pub fn iter(&self) -> std::vec::IntoIter<(&ChangeGroup, &Vec<String>)> {
        self.into_iter()
    }

    pub(crate) fn add(&mut self, change_group: ChangeGroup, item: impl Into<String>) {
        self.0.entry(change_group).or_default().push(item.into());
    }

    pub(crate) fn from_iter<I: IntoIterator<Item = (ChangeGroup, Vec<String>)>>(
        iterable: I,
    ) -> Changes {
        Self(IndexMap::from_iter(iterable))
    }
}

impl<'a> IntoIterator for &'a Changes {
    type Item = (&'a ChangeGroup, &'a Vec<String>);

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().collect::<Vec<_>>().into_iter()
    }
}

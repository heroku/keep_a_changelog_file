use crate::changes::Changes;
use crate::release_link::ReleaseLink;
use crate::ChangeGroup;

/// Tracks upcoming changes. You can move the Unreleased changes into a new [`Release`](struct@crate::release::Release)
/// using [`promote_unreleased`](fn@crate::changelog::Changelog::promote_unreleased).
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct Unreleased {
    /// A link to all unreleased changes.
    pub link: Option<ReleaseLink>,
    /// A grouped list of all unreleased changes.
    pub changes: Changes,
}

impl Unreleased {
    /// Adds the given `item` to the unreleased section under the provided `change_group` heading.
    pub fn add(&mut self, change_group: ChangeGroup, item: impl Into<String>) {
        self.changes.add(change_group, item);
    }
}

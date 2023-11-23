use std::fmt::{Display, Formatter};

/// Changes in a release are grouped into one of several types.
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum ChangeGroup {
    /// For new features.
    Added,
    /// For changes in existing functionality.
    Changed,
    /// For soon-to-be removed features.
    Deprecated,
    /// For any bug fixes.
    Fixed,
    /// For new removed features.
    Removed,
    /// In case of vulnerabilities.
    Security,
}

impl Display for ChangeGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeGroup::Added => write!(f, "Added"),
            ChangeGroup::Changed => write!(f, "Changed"),
            ChangeGroup::Deprecated => write!(f, "Deprecated"),
            ChangeGroup::Removed => write!(f, "Removed"),
            ChangeGroup::Fixed => write!(f, "Fixed"),
            ChangeGroup::Security => write!(f, "Security"),
        }
    }
}

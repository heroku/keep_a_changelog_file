use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

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

#[derive(Debug, Error)]
#[error("Could not parse release tag '{0}'\nExpected: Added | Changed | Deprecated | Removed | Fixed | Security")]
pub struct ParseChangeGroupError(String);

impl FromStr for ChangeGroup {
    type Err = ParseChangeGroupError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "added" => Ok(ChangeGroup::Added),
            "changed" => Ok(ChangeGroup::Changed),
            "deprecated" => Ok(ChangeGroup::Deprecated),
            "removed" => Ok(ChangeGroup::Removed),
            "fixed" => Ok(ChangeGroup::Fixed),
            "security" => Ok(ChangeGroup::Security),
            _ => Err(ParseChangeGroupError(value.to_string())),
        }
    }
}

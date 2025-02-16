#![doc = include_str!("../README.md")]

mod change_group;
mod changelog;
mod changes;
mod parser;
mod release;
mod release_date;
mod release_link;
mod release_tag;
mod release_version;
mod releases;
mod unreleased;

pub use crate::change_group::ChangeGroup;
pub use crate::changelog::Changelog;
pub use crate::changelog::PromoteOptions;
pub use crate::changelog::PromoteUnreleasedError;
pub use crate::changes::Changes;
pub use crate::parser::Diagnostic;
pub use crate::release::Release;
pub use crate::release_date::ParseReleaseDateError;
pub use crate::release_date::ReleaseDate;
pub use crate::release_link::ParseReleaseLinkError;
pub use crate::release_link::ReleaseLink;
pub use crate::release_tag::ReleaseTag;
pub use crate::release_version::ParseVersionError;
pub use crate::release_version::ReleaseVersion;
pub use crate::releases::Releases;
pub use crate::unreleased::Unreleased;

#[doc(hidden)]
#[must_use]
pub fn __printable_syntax_tree(contents: &str) -> String {
    format!("{:?}", parser::parse(contents))
}

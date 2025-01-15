use crate::change_group::ParseChangeGroupError;
use crate::changes::Changes;
use crate::release_tag::ParseReleaseTagError;
use crate::releases::Releases;
use crate::{
    ChangeGroup, ParseReleaseDateError, Release, ReleaseDate, ReleaseLink, ReleaseTag,
    ReleaseVersion, Unreleased,
};
use indexmap::IndexMap;
use lazy_static::lazy_static;
use markdown::mdast::Node;
use markdown::{to_mdast, ParseOptions};
use regex::Regex;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

const CHANGELOG_HEADER: &str = "\
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).";

/// Represents a changelog written in [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.
/// The changelog is a curated, chronologically ordered list of notable changes for each version of a project.
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct Changelog {
    pub title: String,
    pub description: (String, String),
    /// The Unreleased section is always present in the changelog to communicate upcoming changes.
    pub unreleased: Unreleased,
    /// The list of releases
    pub releases: Releases,
}

impl Changelog {
    /// Moves all the changes from the unreleased section of the changelog into a new release which  
    /// is added to the top of the changelog. The version, date, and other fields of the new release
    /// can be customized using the `promote_options` argument. If no date is given in the `promote_options`
    /// then the date will default to the current date.
    ///
    /// This will return the modified changelog or an error if the version being promoted already
    /// exists in the changelog.
    pub fn promote_unreleased(
        &mut self,
        promote_options: &PromoteOptions,
    ) -> Result<(), PromoteUnreleasedError> {
        if self.releases.contains_version(&promote_options.version) {
            Err(PromoteUnreleasedError(promote_options.version.clone()))?;
        }

        let new_release = Release {
            version: promote_options.version.clone(),
            date: promote_options
                .date
                .clone()
                .unwrap_or_else(ReleaseDate::today),
            tag: promote_options.tag.clone(),
            link: promote_options.link.clone(),
            changes: self.unreleased.changes.clone(),
        };

        self.unreleased.changes = Changes::default();

        let mut new_releases: IndexMap<ReleaseVersion, Release> =
            IndexMap::from([(new_release.version.clone(), new_release)]);
        for (release_version, release) in self.releases.clone() {
            new_releases.insert(release_version, release);
        }

        self.releases = Releases::from_iter(new_releases);

        Ok(())
    }
}

impl FromStr for Changelog {
    type Err = ParseChangelogError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_changelog(value).map_err(ParseChangelogError)
    }
}

impl Display for Changelog {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{CHANGELOG_HEADER}")?;

        write!(f, "\n\n## [Unreleased]")?;
        for (change_group, items) in &self.unreleased.changes {
            write!(
                f,
                "\n\n### {change_group}\n\n{}",
                items
                    .iter()
                    .map(|item| format!("- {item}"))
                    .collect::<Vec<String>>()
                    .join("\n")
            )?;
        }

        let mut has_release_with_link = false;

        for (_, release) in &self.releases {
            write!(f, "\n\n## [{}] - {}", release.version, release.date)?;
            if let Some(tag) = &release.tag {
                write!(f, " [{tag}]")?;
            }
            for (change_group, items) in &release.changes {
                write!(
                    f,
                    "\n\n### {change_group}\n\n{}",
                    items
                        .iter()
                        .map(|item| format!("- {item}"))
                        .collect::<Vec<String>>()
                        .join("\n")
                )?;
            }
            if release.link.is_some() {
                has_release_with_link = true;
            }
        }

        if self.unreleased.link.is_some() || has_release_with_link {
            writeln!(f)?;
        }

        if let Some(link) = &self.unreleased.link {
            write!(f, "\n[unreleased]: {link}")?;
        }

        for (_, release) in &self.releases {
            if let Some(link) = &release.link {
                let version = &release.version;
                write!(f, "\n[{version}]: {link}")?;
            }
        }

        writeln!(f)
    }
}

/// Error when promoting unreleased to a version that already exists in the changelog.
#[derive(Debug, Error)]
#[error("Could not promote unreleased to release version {0} because it that version already exists in the changelog")]
pub struct PromoteUnreleasedError(ReleaseVersion);

/// Options for customizing the details of a promoted release.
#[derive(Debug)]
pub struct PromoteOptions {
    version: ReleaseVersion,
    date: Option<ReleaseDate>,
    tag: Option<ReleaseTag>,
    link: Option<ReleaseLink>,
}

impl PromoteOptions {
    /// Construct a new [`PromoteOptions`] instance.
    #[must_use]
    pub fn new(version: ReleaseVersion) -> Self {
        Self {
            version,
            date: None,
            tag: None,
            link: None,
        }
    }

    /// Set the date to use when promoting the release.
    #[must_use]
    pub fn with_date(mut self, date: ReleaseDate) -> Self {
        self.date = Some(date);
        self
    }

    /// Set the release tag to use when promoting the release.
    #[must_use]
    pub fn with_tag(mut self, tag: ReleaseTag) -> Self {
        self.tag = Some(tag);
        self
    }

    /// Set the link to use when promoting the release.
    #[must_use]
    pub fn with_link(mut self, link: ReleaseLink) -> Self {
        self.link = Some(link);
        self
    }
}

#[derive(Debug)]
enum ReleaseHeaderType {
    Unreleased,
    Versioned(ReleaseVersion, ReleaseDate, Option<ReleaseTag>),
}

#[derive(Debug)]
enum ReleaseLinkType {
    Unreleased(ReleaseLink),
    Versioned(ReleaseVersion, ReleaseLink),
}

/// An error that occurred during changelog parsing.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct ParseChangelogError(#[from] ParseChangelogErrorInternal);

#[derive(Debug, Error)]
enum ParseChangelogErrorInternal {
    #[error("Could not parse changelog as markdown\nError: {0}")]
    Markdown(markdown::message::Message),

    #[error("Could not parse change group type from changelog - {0}\nError: {1}")]
    InvalidChangeGroup(String, #[source] ParseChangeGroupError),

    #[error("Release header did not match the expected format\nExpected: [Unreleased] | [<version>] - <yyyy>-<mm>-<dd> | [<version>] - <yyyy>-<mm>-<dd> [<tag>]\nValue: {0}")]
    NoMatchForReleaseHeading(String),

    #[error("Invalid version in release entry - {0}\nValue: {1}\nError: {2}")]
    InvalidVersion(String, String, String),

    #[error("Invalid date in release entry - {0}\nValue: {1}\nError: {2}")]
    InvalidReleaseDate(String, String, #[source] ParseReleaseDateError),

    #[error("Invalid tag in release entry - {0}\nValue: {1}\nError: {2}")]
    InvalidReleaseTag(String, String, #[source] ParseReleaseTagError),
}

// Traverses the changelog written in markdown which has flattened entries that need to be parsed
// and converts those into a nested structure that matches the Keep a Changelog spec. For example,
// given the following markdown doc:
//
// ------------------------------------------
// # Changelog            → (Changelog)
//                        → -
// ## Unreleased          → (Unreleased)
//                        → -
// ## [x.y.z] yyyy-mm-dd  → (Release)
//                        → -
// ### Changed            → (ChangeGroup)
//                        → (Vec)
// - foo                  → (String)
// - bar                  → (String)
//                        → -
// ### Removed            → (ChangeGroup)
//                        → (Vec)
// - baz                  → (String)
// ------------------------------------------
// This would be represented in our Changelog AST as:
//
// Changelog {
//   unreleased: None,
//   releases: [
//     ReleaseEntry {
//       version: x.y.z,
//       date: yyyy-mm-dd,
//       tag: None,
//       contents: ReleaseContents {
//         "Changed": ["foo", "bar"],
//         "Removed": ["baz"]
//       }
//     }
//   ]
// }
#[allow(clippy::too_many_lines)]
fn parse_changelog(input: &str) -> Result<Changelog, ParseChangelogErrorInternal> {
    let changelog_ast =
        to_mdast(input, &ParseOptions::default()).map_err(ParseChangelogErrorInternal::Markdown)?;

    let is_release_entry_heading = is_heading_of_depth(2);
    let is_change_group_heading = is_heading_of_depth(3);
    let is_list_node = |node: &Node| matches!(node, Node::List(_));
    let is_definition = |node: &Node| matches!(node, Node::Definition(_));

    let mut unreleased = None;
    let mut unreleased_link = None;
    let mut releases = IndexMap::new();
    let mut release_links = HashMap::new();

    if let Node::Root(root) = changelog_ast {
        // the peekable iterator here makes it easier to decide when to traverse to the next sibling
        // node in the markdown AST to construct our nested structure
        let mut root_iter = root.children.into_iter().peekable();
        while root_iter.peek().is_some() {
            if let Some(release_heading_node) = root_iter.next_if(&is_release_entry_heading) {
                let release_entry_type = parse_release_heading(release_heading_node.to_string())?;
                let mut changes: IndexMap<ChangeGroup, Vec<String>> = IndexMap::new();

                while root_iter.peek().is_some_and(&is_change_group_heading) {
                    if let Some(change_group_node) = root_iter.next() {
                        let change_group = change_group_node
                            .to_string()
                            .parse::<ChangeGroup>()
                            .map_err(|e| {
                                ParseChangelogErrorInternal::InvalidChangeGroup(
                                    change_group_node.to_string(),
                                    e,
                                )
                            })?;

                        while root_iter.peek().is_some_and(is_list_node) {
                            if let Some(list_node) = root_iter.next() {
                                if let Some(list_items) = list_node.children() {
                                    for list_item in list_items {
                                        if matches!(list_item, Node::ListItem(_)) {
                                            if let Some(position) = list_item.position() {
                                                let text = input
                                                    [position.start.offset..position.end.offset]
                                                    .trim_start_matches(['-', '*', ' '])
                                                    .trim_end()
                                                    .to_string();
                                                match change_group {
                                                    ChangeGroup::Added => {
                                                        changes
                                                            .entry(ChangeGroup::Added)
                                                            .or_default()
                                                            .push(text);
                                                    }
                                                    ChangeGroup::Changed => {
                                                        changes
                                                            .entry(ChangeGroup::Changed)
                                                            .or_default()
                                                            .push(text);
                                                    }
                                                    ChangeGroup::Deprecated => {
                                                        changes
                                                            .entry(ChangeGroup::Deprecated)
                                                            .or_default()
                                                            .push(text);
                                                    }
                                                    ChangeGroup::Fixed => {
                                                        changes
                                                            .entry(ChangeGroup::Fixed)
                                                            .or_default()
                                                            .push(text);
                                                    }
                                                    ChangeGroup::Removed => {
                                                        changes
                                                            .entry(ChangeGroup::Removed)
                                                            .or_default()
                                                            .push(text);
                                                    }
                                                    ChangeGroup::Security => {
                                                        changes
                                                            .entry(ChangeGroup::Security)
                                                            .or_default()
                                                            .push(text);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                match release_entry_type {
                    ReleaseHeaderType::Unreleased => {
                        unreleased = Some(Unreleased {
                            changes: Changes::from_iter(changes.into_iter()),
                            link: None,
                        });
                    }
                    ReleaseHeaderType::Versioned(version, date, tag) => {
                        releases.insert(
                            version.clone(),
                            Release {
                                version,
                                date,
                                tag,
                                link: None,
                                changes: Changes::from_iter(changes.into_iter()),
                            },
                        );
                    }
                }
            } else if let Some(definition_node) = root_iter.next_if(is_definition) {
                if let Node::Definition(definition) = definition_node {
                    if let Some(release_link_type) =
                        parse_release_link_type(&definition.identifier, &definition.url)
                    {
                        match release_link_type {
                            ReleaseLinkType::Unreleased(uri) => unreleased_link = Some(uri),
                            ReleaseLinkType::Versioned(version, uri) => {
                                release_links.insert(version, uri);
                            }
                        }
                    }
                }
            } else {
                root_iter.next();
            }
        }
    }

    if let Some(ref mut next_release) = unreleased {
        next_release.link = unreleased_link;
    }

    for (version, link) in release_links {
        if let Some(release) = releases.get_mut(&version) {
            release.link = Some(link);
        }
    }

    Ok(Changelog {
        title: String::new(),
        description: (String::new(), String::new()),
        unreleased: unreleased.unwrap_or_default(),
        releases: Releases::from_iter(releases),
    })
}

fn is_heading_of_depth(depth: u8) -> impl Fn(&Node) -> bool {
    move |node: &Node| {
        if let Node::Heading(heading) = node {
            return heading.depth == depth;
        }
        false
    }
}

const UNRELEASED: &str = "unreleased";
const VERSION_CAPTURE: &str = r"(?P<version>\d+\.\d+\.\d+)";
const RELEASE_DATE_CAPTURE: &str = r"(?P<release_date>\d{4}-\d{2}-\d{2})";
const TAG_CAPTURE: &str = r"(?P<tag>.+)";

lazy_static! {
    static ref UNRELEASED_HEADER: Regex =
        Regex::new(&format!(r"(?i)^\[?{UNRELEASED}]?$")).expect("Should be a valid regex");
    static ref VERSIONED_RELEASE_HEADER: Regex = Regex::new(&format!(
        r"^\[?{VERSION_CAPTURE}]?\s+-\s+{RELEASE_DATE_CAPTURE}(?:\s+\[{TAG_CAPTURE}])?$"
    ))
    .expect("Should be a valid regex");
}

fn parse_release_heading(
    heading: String,
) -> Result<ReleaseHeaderType, ParseChangelogErrorInternal> {
    if UNRELEASED_HEADER.is_match(&heading) {
        return Ok(ReleaseHeaderType::Unreleased);
    }

    if let Some(captures) = VERSIONED_RELEASE_HEADER.captures(&heading) {
        let release_version = captures["version"].parse::<ReleaseVersion>().map_err(|e| {
            ParseChangelogErrorInternal::InvalidVersion(
                heading.clone(),
                captures["version"].to_string(),
                e.to_string(),
            )
        })?;

        let release_date = captures["release_date"]
            .parse::<ReleaseDate>()
            .map_err(|e| {
                ParseChangelogErrorInternal::InvalidReleaseDate(
                    heading.clone(),
                    captures["release_date"].to_string(),
                    e,
                )
            })?;

        let release_tag = if let Some(tag_value) = captures.name("tag") {
            Some(tag_value.as_str().parse::<ReleaseTag>().map_err(|e| {
                ParseChangelogErrorInternal::InvalidReleaseTag(
                    heading.clone(),
                    tag_value.as_str().to_string(),
                    e,
                )
            })?)
        } else {
            None
        };

        Ok(ReleaseHeaderType::Versioned(
            release_version,
            release_date,
            release_tag,
        ))
    } else {
        Err(ParseChangelogErrorInternal::NoMatchForReleaseHeading(
            heading,
        ))
    }
}

fn parse_release_link_type(version: &str, url: &str) -> Option<ReleaseLinkType> {
    let parsed_url = url.parse();
    if version.to_lowercase() == UNRELEASED {
        parsed_url.map(ReleaseLinkType::Unreleased).ok()
    } else if let Ok(version) = version.parse::<ReleaseVersion>() {
        parsed_url
            .map(|uri| ReleaseLinkType::Versioned(version, uri))
            .ok()
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]
    use super::*;

    macro_rules! assert_err_matches {
        ($left:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
            match $left {
                Ok(value) => {
                    panic!("Expected Err but was Ok({value:?})")
                }
                Err(e) => match e {
                    $( $pattern )|+ $( if $guard )? => {}
                    error => panic!("Expected to match but was {error:?}"),
                },
            }
        };
    }

    #[test]
    fn test_invalid_change_group() {
        let changelog: Result<Changelog, _> = parse_changelog(&format!(
            "{CHANGELOG_HEADER}
## Unreleased

### Invalid

- Some change        
        "
        ));
        assert_err_matches!(changelog, ParseChangelogErrorInternal::InvalidChangeGroup(group, _) if group == "Invalid");
    }

    #[test]
    fn test_not_a_valid_release_heading() {
        let release_heading = "Not a release header";
        let changelog: Result<Changelog, _> =
            parse_changelog(&format!("{CHANGELOG_HEADER}\n\n## {release_heading}"));
        assert_err_matches!(changelog, ParseChangelogErrorInternal::NoMatchForReleaseHeading(heading) if heading == release_heading);
    }

    #[test]
    fn test_invalid_release_version() {
        let release_heading = "[00.01.02] - 2023-01-01";
        let changelog: Result<Changelog, _> =
            parse_changelog(&format!("{CHANGELOG_HEADER}\n\n## {release_heading}"));
        assert_err_matches!(changelog, ParseChangelogErrorInternal::InvalidVersion(heading, version, _) if heading == release_heading && version == "00.01.02");
    }

    #[test]
    fn test_invalid_release_date() {
        let release_heading = "[0.1.2] - 9999-99-99";
        let changelog: Result<Changelog, _> =
            parse_changelog(&format!("{CHANGELOG_HEADER}\n\n## {release_heading}"));
        assert_err_matches!(changelog, ParseChangelogErrorInternal::InvalidReleaseDate(heading, release_date, _) if heading == release_heading && release_date == "9999-99-99");
    }

    #[test]
    fn test_valid_release_tag() {
        let changelog: Changelog =
            format!("{CHANGELOG_HEADER}\n\n## [0.1.2] - 2023-01-01 [YANKED]")
                .parse()
                .unwrap();
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.1.2".parse::<ReleaseVersion>().unwrap())
                .unwrap()
                .tag,
            Some(ReleaseTag::Yanked)
        );
    }

    #[test]
    fn test_invalid_release_tag() {
        let release_heading = "[0.1.2] - 2023-01-01 [UNKNOWN TAG]";
        let changelog: Result<Changelog, _> =
            parse_changelog(&format!("{CHANGELOG_HEADER}\n\n## {release_heading}"));
        assert_err_matches!(changelog, ParseChangelogErrorInternal::InvalidReleaseTag(heading, tag, _) if heading == release_heading && tag == "UNKNOWN TAG");
    }
}

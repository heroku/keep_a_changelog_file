use chrono::{DateTime, LocalResult, TimeZone, Utc};
use indexmap::IndexMap;
use lazy_static::lazy_static;
use markdown::mdast::Node;
use markdown::{to_mdast, ParseOptions};
use regex::Regex;
use semver::Version;
use std::collections::HashMap;
use std::num::ParseIntError;
use std::str::FromStr;
use uriparse::URI;

const UNRELEASED: &str = "unreleased";
const VERSION_CAPTURE: &str = r"(?P<version>\d+\.\d+\.\d+)";
const YEAR_CAPTURE: &str = r"(?P<year>\d{4})";
const MONTH_CAPTURE: &str = r"(?P<month>\d{2})";
const DAY_CAPTURE: &str = r"(?P<day>\d{2})";
const TAG_CAPTURE: &str = r"(?P<tag>.+)";

lazy_static! {
    static ref UNRELEASED_HEADER: Regex =
        Regex::new(&format!(r"(?i)^\[?{UNRELEASED}]?$")).expect("Should be a valid regex");

    static ref VERSIONED_RELEASE_HEADER: Regex = Regex::new(&format!(
        r"^\[?{VERSION_CAPTURE}]?\s+-\s+{YEAR_CAPTURE}[-/]{MONTH_CAPTURE}[-/]{DAY_CAPTURE}(?:\s+\[{TAG_CAPTURE}])?$"
    ))
    .expect("Should be a valid regex");
}

const CHANGELOG_HEADER: &str = "# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), \
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).";

#[derive(Debug, Eq, PartialEq)]
pub struct Changelog {
    pub unreleased: NextRelease,
    releases: IndexMap<Version, Release>,
}

impl Changelog {
    pub fn release(&self, version: &Version) -> Option<&Release> {
        self.releases.get(version)
    }

    pub fn releases(&self) {
        todo!()
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct NextRelease {
    pub link: Option<URI<'static>>,
    pub details: ReleaseDetails,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Release {
    pub version: Version,
    pub date: DateTime<Utc>,
    pub tag: Option<ReleaseTag>,
    pub link: Option<URI<'static>>,
    pub details: ReleaseDetails,
}

impl Release {
    pub fn new(version: Version, date: DateTime<Utc>) -> Self {
        Self {
            version,
            date,
            tag: None,
            link: None,
            details: ReleaseDetails::default(),
        }
    }

    pub fn link(mut self, value: URI<'static>) -> Self {
        self.link = Some(value);
        self
    }

    pub fn release_tag(mut self, release_tag: ReleaseTag) -> Self {
        self.tag = Some(release_tag);
        self
    }

    pub fn details(mut self, details: ReleaseDetails) -> Self {
        self.details = details;
        self
    }

    pub fn with_details(mut self, modify_fn: impl FnOnce(&mut ReleaseDetails)) -> Self {
        modify_fn(&mut self.details);
        self
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ReleaseTag {
    Yanked,
    NoChanges,
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct ReleaseDetails {
    added: Vec<String>,
    changed: Vec<String>,
    deprecated: Vec<String>,
    fixed: Vec<String>,
    removed: Vec<String>,
    security: Vec<String>,
}

impl ReleaseDetails {
    fn added_item<S: Into<String>>(mut self, item: S) -> Self {
        append_release_detail_items(&mut self.added, [item]);
        self
    }

    fn added_items<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        append_release_detail_items(&mut self.added, items);
        self
    }

    fn changed_item<S: Into<String>>(mut self, item: S) -> Self {
        append_release_detail_items(&mut self.changed, [item]);
        self
    }

    fn changed_items<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        append_release_detail_items(&mut self.changed, items);
        self
    }

    fn deprecated_item<S: Into<String>>(mut self, item: S) -> Self {
        append_release_detail_items(&mut self.deprecated, [item]);
        self
    }

    fn deprecated_items<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        append_release_detail_items(&mut self.deprecated, items);
        self
    }

    fn fixed_item<S: Into<String>>(mut self, item: S) -> Self {
        append_release_detail_items(&mut self.fixed, [item]);
        self
    }

    fn fixed_items<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        append_release_detail_items(&mut self.fixed, items);
        self
    }

    fn removed_item<S: Into<String>>(mut self, item: S) -> Self {
        append_release_detail_items(&mut self.removed, [item]);
        self
    }

    fn removed_items<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        append_release_detail_items(&mut self.removed, items);
        self
    }

    fn security_item<S: Into<String>>(mut self, item: S) -> Self {
        append_release_detail_items(&mut self.security, [item]);
        self
    }

    fn security_items<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        append_release_detail_items(&mut self.security, items);
        self
    }
}

fn append_release_detail_items<I, S>(vec: &mut Vec<String>, items: I)
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    for item in items {
        vec.push(item.into());
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum ChangeGroup {
    Added,
    Changed,
    Deprecated,
    Fixed,
    Removed,
    Security,
}

#[derive(Debug)]
enum ReleaseHeaderType {
    Unreleased,
    Versioned(Version, DateTime<Utc>, Option<ReleaseTag>),
}

#[derive(Debug)]
enum ReleaseLink {
    Unreleased(URI<'static>),
    Versioned(Version, URI<'static>),
}

#[derive(Debug, thiserror::Error)]
pub enum ParseChangelogError {
    #[error("Could not parse changelog as markdown\nError: {0}")]
    Markdown(String),
    #[error("Could not parse change group type from changelog\nExpected: Added | Changed | Deprecated | Removed | Fixed | Security\nValue: {0}")]
    InvalidChangeGroup(String),
    #[error("Release header did not match the expected format\nExpected: [Unreleased] | [<version>] - <yyyy>-<mm>-<dd> | [<version>] - <yyyy>-<mm>-<dd> [<tag>]\nValue: {0}")]
    NoMatchForReleaseHeading(String),
    #[error("Invalid semver version in release entry - {0}\nValue: {1}\nError: {2}")]
    Version(String, String, #[source] semver::Error),
    #[error("Invalid year in release entry - {0}\nValue: {1}\nError: {2}")]
    ReleaseEntryYear(String, String, #[source] ParseIntError),
    #[error("Invalid month in release entry - {0}\nValue: {1}\nError: {2}")]
    ReleaseEntryMonth(String, String, #[source] ParseIntError),
    #[error("Invalid day in release entry - {0}\nValue: {1}\nError: {2}")]
    ReleaseEntryDay(String, String, #[source] ParseIntError),
    #[error("Invalid date in release entry - {0}\nValue: {1}-{2}-{3}")]
    InvalidReleaseDate(String, i32, u32, u32),
    #[error("Ambiguous date in release entry - {0}\nValue: {1}-{2}-{3}")]
    AmbiguousReleaseDate(String, i32, u32, u32),
    #[error(
        "Could not parse release tag from changelog\nExpected: YANKED | NO CHANGES\nValue: {1}"
    )]
    InvalidReleaseTag(String, String),
}

// Traverses the changelog written in markdown which has flattened entries that need to be parsed
// and converts those into a nested structure that matches the Keep a Changelog spec. For example,
// given the following markdown doc:
//
// ------------------------------------------
// # Changelog            → (Changelog)
//                        → -
// ## Unreleased          → (ReleaseEntry::Unreleased)
//                        → (ReleaseContents)
// ## [x.y.z] yyyy-mm-dd  → (ReleaseEntry::Versioned)
//                        → (ReleaseContents)
// ### Changed            → (ChangeGroup)
//                        → (List)
// - foo                  → (List Item)
// - bar                  → (List Item)
//                        → -
// ### Removed            → (ChangeGroup)
//                        → (List)
// - baz                  → (List Item)
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
pub(crate) fn parse_changelog(input: &str) -> Result<Changelog, ParseChangelogError> {
    let changelog_ast =
        to_mdast(input, &ParseOptions::default()).map_err(ParseChangelogError::Markdown)?;

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
                let mut release_details = ReleaseDetails::default();

                while root_iter.peek().is_some_and(&is_change_group_heading) {
                    let change_group_node = root_iter.next().expect(
                        "This should be a change group heading node since we already peeked at it",
                    );
                    let change_group = parse_change_group_heading(change_group_node.to_string())?;

                    while root_iter.peek().is_some_and(is_list_node) {
                        let list_node = root_iter
                            .next()
                            .expect("This should be a list node since we already peeked at it");
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
                                            ChangeGroup::Added => release_details.added.push(text),
                                            ChangeGroup::Changed => {
                                                release_details.changed.push(text)
                                            }
                                            ChangeGroup::Deprecated => {
                                                release_details.deprecated.push(text)
                                            }
                                            ChangeGroup::Fixed => release_details.fixed.push(text),
                                            ChangeGroup::Removed => {
                                                release_details.removed.push(text)
                                            }
                                            ChangeGroup::Security => {
                                                release_details.security.push(text)
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
                        unreleased = Some(NextRelease {
                            details: release_details,
                            link: None,
                        })
                    }
                    ReleaseHeaderType::Versioned(version, date, tag) => {
                        releases.insert(
                            version.clone(),
                            Release {
                                version,
                                date,
                                tag,
                                link: None,
                                details: release_details,
                            },
                        );
                    }
                }
            } else if let Some(definition_node) = root_iter.next_if(is_definition) {
                if let Node::Definition(definition) = definition_node {
                    if let Some(release_link) =
                        parse_release_link(definition.identifier, definition.url)
                    {
                        match release_link {
                            ReleaseLink::Unreleased(uri) => unreleased_link = Some(uri),
                            ReleaseLink::Versioned(version, uri) => {
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
        unreleased: unreleased.unwrap_or_default(),
        releases,
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

fn parse_release_heading(heading: String) -> Result<ReleaseHeaderType, ParseChangelogError> {
    if UNRELEASED_HEADER.is_match(&heading) {
        return Ok(ReleaseHeaderType::Unreleased);
    }

    if let Some(captures) = VERSIONED_RELEASE_HEADER.captures(&heading) {
        let version = captures["version"]
            .parse::<semver::Version>()
            .map_err(|e| {
                ParseChangelogError::Version(heading.clone(), captures["version"].to_string(), e)
            })?;

        let year = captures["year"].parse::<i32>().map_err(|e| {
            ParseChangelogError::ReleaseEntryYear(heading.clone(), captures["year"].to_string(), e)
        })?;

        let month = captures["month"].parse::<u32>().map_err(|e| {
            ParseChangelogError::ReleaseEntryMonth(
                heading.clone(),
                captures["month"].to_string(),
                e,
            )
        })?;

        let day = captures["day"].parse::<u32>().map_err(|e| {
            ParseChangelogError::ReleaseEntryDay(heading.clone(), captures["day"].to_string(), e)
        })?;

        let date = match Utc.with_ymd_and_hms(year, month, day, 0, 0, 0) {
            LocalResult::None => Err(ParseChangelogError::InvalidReleaseDate(
                heading.clone(),
                year,
                month,
                day,
            )),
            LocalResult::Single(value) => Ok(value),
            LocalResult::Ambiguous(_, _) => Err(ParseChangelogError::AmbiguousReleaseDate(
                heading.clone(),
                year,
                month,
                day,
            )),
        }?;

        let tag = if let Some(tag_value) = captures.name("tag") {
            match tag_value.as_str().to_lowercase().as_str() {
                "no changes" => Ok(Some(ReleaseTag::NoChanges)),
                "yanked" => Ok(Some(ReleaseTag::Yanked)),
                _ => Err(ParseChangelogError::InvalidReleaseTag(
                    heading.clone(),
                    captures["tag"].to_string(),
                )),
            }?
        } else {
            None
        };

        Ok(ReleaseHeaderType::Versioned(version, date, tag))
    } else {
        Err(ParseChangelogError::NoMatchForReleaseHeading(heading))
    }
}

fn parse_change_group_heading(heading: String) -> Result<ChangeGroup, ParseChangelogError> {
    match heading.trim().to_lowercase().as_str() {
        "added" => Ok(ChangeGroup::Added),
        "changed" => Ok(ChangeGroup::Changed),
        "deprecated" => Ok(ChangeGroup::Deprecated),
        "removed" => Ok(ChangeGroup::Removed),
        "fixed" => Ok(ChangeGroup::Fixed),
        "security" => Ok(ChangeGroup::Security),
        _ => Err(ParseChangelogError::InvalidChangeGroup(heading)),
    }
}

fn parse_release_link(version: String, url: String) -> Option<ReleaseLink> {
    let parsed_url = URI::try_from(url.as_str()).map(URI::into_owned);
    if version.to_lowercase() == UNRELEASED {
        parsed_url.map(ReleaseLink::Unreleased).ok()
    } else if let Ok(version) = version.parse::<Version>() {
        parsed_url
            .map(|uri| ReleaseLink::Versioned(version, uri))
            .ok()
    } else {
        None
    }
}

impl FromStr for Changelog {
    type Err = ParseChangelogError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        parse_changelog(value)
    }
}

// impl Display for Changelog {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "{}",
//             r"
// # Changelog
// All notable changes to this project will be documented in this file.
// The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
// and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
//         "
//             .trim()
//         )?;
//
//         if let Some(unreleased) = &self.unreleased {
//             write!(f, "\n\n## [Unreleased]\n\n{unreleased}")?;
//         } else {
//             write!(f, "\n\n## [Unreleased]")?;
//         }
//
//         for entry in &self.releases {
//             write!(
//                 f,
//                 "\n\n## [{}] - {}",
//                 entry.version,
//                 entry.date.format("%Y-%m-%d")
//             )?;
//             if let Some(tag) = &entry.tag {
//                 write!(f, " [{tag}]")?;
//             }
//             write!(f, "\n\n{}", entry.contents)?;
//         }
//
//         writeln!(f)
//     }
// }
//
// impl Display for ReleaseContents {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         for (change_group, items) in &self.change_groups {
//             if !items.is_empty() {
//                 write!(f, "### {change_group}\n\n")?;
//                 for item in items {
//                     writeln!(f, "- {item}")?;
//                 }
//             }
//         }
//         writeln!(f)
//     }
// }
//
// impl Display for ReleaseTag {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             ReleaseTag::Yanked => write!(f, "YANKED"),
//             ReleaseTag::NoChanges => write!(f, "NO CHANGES"),
//         }
//     }
// }
//
// impl Display for ChangeGroup {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             ChangeGroup::Added => write!(f, "Added"),
//             ChangeGroup::Changed => write!(f, "Changed"),
//             ChangeGroup::Deprecated => write!(f, "Deprecated"),
//             ChangeGroup::Removed => write!(f, "Removed"),
//             ChangeGroup::Fixed => write!(f, "Fixed"),
//             ChangeGroup::Security => write!(f, "Security"),
//         }
//     }
// }

#[cfg(test)]
mod test {
    use super::*;

    fn version(version: &str) -> Version {
        version.parse().unwrap()
    }

    fn date(yyyy_mm_dd: &str) -> DateTime<Utc> {
        format!("{yyyy_mm_dd}T00:00:00Z").parse().unwrap()
    }

    fn uri(uri: &str) -> URI<'static> {
        URI::try_from(uri).map(URI::into_owned).unwrap()
    }

    #[test]
    fn test_release_v1_1_1() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        assert_eq!(
            changelog.release(&version("1.1.1")).unwrap(),
            &Release::new(version("1.1.1"), date("2023-03-05"))
                .link(uri(
                    "https://github.com/olivierlacan/keep-a-changelog/compare/v1.1.0...v1.1.1"
                ))
            .with_details(|mut details| {
                details
                    .added_items([
                    "Arabic translation (#444).",
                                        "v1.1 French translation.",
                                        "v1.1 Dutch translation (#371).",
                                        "v1.1 Russian translation (#410).",
                                        "v1.1 Japanese translation (#363).",
                                        "v1.1 Norwegian Bokmål translation (#383).",
                                        "v1.1 \"Inconsistent Changes\" Turkish translation (#347).",
                                        "Default to most recent versions available for each languages",
                                        "Display count of available translations (26 to date!)",
                                        "Centralize all links into `/data/links.json` so they can be updated easily",
                ])
                    .fixed_items([
                                       "Improve French translation (#377).",
                                       "Improve id-ID translation (#416).",
                                       "Improve Persian translation (#457).",
                                       "Improve Russian translation (#408).",
                                       "Improve Swedish title (#419).",
                                       "Improve zh-CN translation (#359).",
                                       "Improve French translation (#357).",
                                       "Improve zh-TW translation (#360, #355).",
                                       "Improve Spanish (es-ES) transltion (#362).",
                                       "Foldout menu in Dutch translation (#371).",
                                       "Missing periods at the end of each change (#451).",
                                       "Fix missing logo in 1.1 pages",
                                       "Display notice when translation isn't for most recent version",
                                       "Various broken links, page versions, and indentations.",
                    ])
                .changed_item("Upgrade dependencies: Ruby 3.2.1, Middleman, etc.")
                    .removed_items(["Unused normalize.css file", "Identical links assigned in each translation file","Duplicate index file for the english version"]);
            })
        )
    }

    const KEEP_A_CHANGELOG: &str = r#"# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.1] - 2023-03-05

### Added

- Arabic translation (#444).
- v1.1 French translation.
- v1.1 Dutch translation (#371).
- v1.1 Russian translation (#410).
- v1.1 Japanese translation (#363).
- v1.1 Norwegian Bokmål translation (#383).
- v1.1 "Inconsistent Changes" Turkish translation (#347).
- Default to most recent versions available for each languages
- Display count of available translations (26 to date!)
- Centralize all links into `/data/links.json` so they can be updated easily

### Fixed

- Improve French translation (#377).
- Improve id-ID translation (#416).
- Improve Persian translation (#457).
- Improve Russian translation (#408).
- Improve Swedish title (#419).
- Improve zh-CN translation (#359).
- Improve French translation (#357).
- Improve zh-TW translation (#360, #355).
- Improve Spanish (es-ES) transltion (#362).
- Foldout menu in Dutch translation (#371).
- Missing periods at the end of each change (#451).
- Fix missing logo in 1.1 pages
- Display notice when translation isn't for most recent version
- Various broken links, page versions, and indentations.

### Changed

- Upgrade dependencies: Ruby 3.2.1, Middleman, etc.

### Removed

- Unused normalize.css file
- Identical links assigned in each translation file
- Duplicate index file for the english version

## [1.1.0] - 2019-02-15

### Added

- Danish translation (#297).
- Georgian translation from (#337).
- Changelog inconsistency section in Bad Practices.

### Fixed

- Italian translation (#332).
- Indonesian translation (#336).

## [1.0.0] - 2017-06-20

### Added

- New visual identity by [@tylerfortune8](https://github.com/tylerfortune8).
- Version navigation.
- Links to latest released version in previous versions.
- "Why keep a changelog?" section.
- "Who needs a changelog?" section.
- "How do I make a changelog?" section.
- "Frequently Asked Questions" section.
- New "Guiding Principles" sub-section to "How do I make a changelog?".
- Simplified and Traditional Chinese translations from [@tianshuo](https://github.com/tianshuo).
- German translation from [@mpbzh](https://github.com/mpbzh) & [@Art4](https://github.com/Art4).
- Italian translation from [@azkidenz](https://github.com/azkidenz).
- Swedish translation from [@magol](https://github.com/magol).
- Turkish translation from [@emreerkan](https://github.com/emreerkan).
- French translation from [@zapashcanon](https://github.com/zapashcanon).
- Brazilian Portuguese translation from [@Webysther](https://github.com/Webysther).
- Polish translation from [@amielucha](https://github.com/amielucha) & [@m-aciek](https://github.com/m-aciek).
- Russian translation from [@aishek](https://github.com/aishek).
- Czech translation from [@h4vry](https://github.com/h4vry).
- Slovak translation from [@jkostolansky](https://github.com/jkostolansky).
- Korean translation from [@pierceh89](https://github.com/pierceh89).
- Croatian translation from [@porx](https://github.com/porx).
- Persian translation from [@Hameds](https://github.com/Hameds).
- Ukrainian translation from [@osadchyi-s](https://github.com/osadchyi-s).

### Changed

- Start using "changelog" over "change log" since it's the common usage.
- Start versioning based on the current English version at 0.3.0 to help
  translation authors keep things up-to-date.
- Rewrite "What makes unicorns cry?" section.
- Rewrite "Ignoring Deprecations" sub-section to clarify the ideal
  scenario.
- Improve "Commit log diffs" sub-section to further argument against
  them.
- Merge "Why can’t people just use a git log diff?" with "Commit log
    diffs".
- Fix typos in Simplified Chinese and Traditional Chinese translations.
- Fix typos in Brazilian Portuguese translation.
- Fix typos in Turkish translation.
- Fix typos in Czech translation.
- Fix typos in Swedish translation.
- Improve phrasing in French translation.
- Fix phrasing and spelling in German translation.

### Removed

- Section about "changelog" vs "CHANGELOG".

## [0.3.0] - 2015-12-03

### Added

- RU translation from [@aishek](https://github.com/aishek).
- pt-BR translation from [@tallesl](https://github.com/tallesl).
- es-ES translation from [@ZeliosAriex](https://github.com/ZeliosAriex).

## [0.2.0] - 2015-10-06

### Changed

- Remove exclusionary mentions of "open source" since this project can
  benefit both "open" and "closed" source projects equally.

## [0.1.0] - 2015-10-06

### Added

- Answer "Should you ever rewrite a change log?".

### Changed

- Improve argument against commit logs.
- Start following [SemVer](https://semver.org) properly.

## [0.0.8] - 2015-02-17

### Changed

- Update year to match in every README example.
- Reluctantly stop making fun of Brits only, since most of the world
  writes dates in a strange way.

### Fixed

- Fix typos in recent README changes.
- Update outdated unreleased diff link.

## [0.0.7] - 2015-02-16

### Added

- Link, and make it obvious that date format is ISO 8601.

### Changed

- Clarified the section on "Is there a standard change log format?".

### Fixed

- Fix Markdown links to tag comparison URL with footnote-style links.

## [0.0.6] - 2014-12-12

### Added

- README section on "yanked" releases.

## [0.0.5] - 2014-08-09

### Added

- Markdown links to version tags on release headings.
- Unreleased section to gather unreleased changes and encourage note
  keeping prior to releases.

## [0.0.4] - 2014-08-09

### Added

- Better explanation of the difference between the file ("CHANGELOG")
  and its function "the change log".

### Changed

- Refer to a "change log" instead of a "CHANGELOG" throughout the site
  to differentiate between the file and the purpose of the file — the
  logging of changes.

### Removed

- Remove empty sections from CHANGELOG, they occupy too much space and
  create too much noise in the file. People will have to assume that the
  missing sections were intentionally left out because they contained no
  notable changes.

## [0.0.3] - 2014-08-09

### Added

- "Why should I care?" section mentioning The Changelog podcast.

## [0.0.2] - 2014-07-10

### Added

- Explanation of the recommended reverse chronological release ordering.

## [0.0.1] - 2014-05-31

### Added

- This CHANGELOG file to hopefully serve as an evolving example of a
  standardized open source project CHANGELOG.
- CNAME file to enable GitHub Pages custom domain.
- README now contains answers to common questions about CHANGELOGs.
- Good examples and basic guidelines, including proper date formatting.
- Counter-examples: "What makes unicorns cry?".

[unreleased]: https://github.com/olivierlacan/keep-a-changelog/compare/v1.1.1...HEAD
[1.1.1]: https://github.com/olivierlacan/keep-a-changelog/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/olivierlacan/keep-a-changelog/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.3.0...v1.0.0
[0.3.0]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.8...v0.1.0
[0.0.8]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.7...v0.0.8
[0.0.7]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.6...v0.0.7
[0.0.6]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.5...v0.0.6
[0.0.5]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.4...v0.0.5
[0.0.4]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/olivierlacan/keep-a-changelog/releases/tag/v0.0.1
"#;
}

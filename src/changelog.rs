use crate::changes::Changes;
use crate::parser::{
    parse, Child, ReleaseLinkType, Tree, TreeKind, ABOUT_FORMAT_TEXT, CHANGELOG_TITLE,
    NOTABLE_CHANGES_TEXT,
};
use crate::releases::Releases;
use crate::{
    ChangeGroup, Diagnostic, Release, ReleaseDate, ReleaseLink, ReleaseTag, ReleaseVersion,
    Unreleased,
};
use indexmap::IndexMap;
use markdown::mdast::Node;
use mdast_util_to_markdown::to_markdown;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

/// Represents a changelog written in [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.
/// The changelog is a curated, chronologically ordered list of notable changes for each version of a project.
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct Changelog {
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
    type Err = Vec<Diagnostic>;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let tree = parse(value);
        let diagnostics = tree.get_diagnostics();

        if diagnostics.is_empty() {
            let mut releases = IndexMap::new();
            let mut unreleased_link = None;
            let mut release_links = HashMap::new();

            for tree in tree.tree_children() {
                if let TreeKind::ReleaseLink(release_link_type) = &tree.kind {
                    match release_link_type {
                        ReleaseLinkType::Unreleased(link) => {
                            unreleased_link = Some(link.clone());
                        }
                        ReleaseLinkType::Versioned(version, link) => {
                            release_links.insert(version, link.clone());
                        }
                    }
                }
            }

            let unreleased_tree =
                expect_one_tree(&tree, |child_tree| child_tree.kind == TreeKind::Unreleased)?;

            for tree in tree.tree_children() {
                if let TreeKind::Release = tree.kind {
                    let release_header_tree = expect_one_tree(tree, |child_tree| {
                        matches!(child_tree.kind, TreeKind::ReleaseHeader(_, _, _))
                    })?;
                    if let TreeKind::ReleaseHeader(version, date, tag) = &release_header_tree.kind {
                        releases.insert(
                            version.clone(),
                            Release {
                                version: version.clone(),
                                date: date.clone(),
                                tag: tag.clone(),
                                changes: Changes::from_iter(extract_change_groups(tree)?),
                                link: release_links.get(&version).cloned(),
                            },
                        );
                    }
                }
            }

            Ok(Changelog {
                unreleased: Unreleased {
                    link: unreleased_link,
                    changes: Changes::from_iter(extract_change_groups(unreleased_tree)?),
                },
                releases: Releases::from_iter(releases),
            })
        } else {
            Err(diagnostics)
        }
    }
}

fn expect_one_tree(tree: &Tree, filter: fn(&Tree) -> bool) -> Result<&Tree, Vec<Diagnostic>> {
    let found = tree
        .tree_children()
        .filter(|child_tree| filter(child_tree))
        .collect::<Vec<_>>();
    if found.len() == 1 {
        if let Some(found) = found.first() {
            return Ok(found);
        }
    }
    todo!()
}

fn expect_one_markdown_node(
    tree: &Tree,
    filter: fn(&Node) -> bool,
) -> Result<&Node, Vec<Diagnostic>> {
    let found = tree
        .children
        .iter()
        .filter_map(|child| {
            if let Child::Markdown(node) = child {
                if filter(node) {
                    return Some(node);
                }
            }
            None
        })
        .collect::<Vec<_>>();
    if found.len() == 1 {
        if let Some(found) = found.first() {
            return Ok(found);
        }
    }
    todo!()
}

fn extract_change_groups(tree: &Tree) -> Result<Vec<(ChangeGroup, Vec<String>)>, Vec<Diagnostic>> {
    let mut results = vec![];
    for tree in tree.tree_children() {
        if let TreeKind::ChangeGroup = tree.kind {
            let mut changes = vec![];
            let change_group_tree = expect_one_tree(tree, |child_tree| {
                matches!(child_tree.kind, TreeKind::ChangeGroupHeader(_))
            })?;
            let change_group_list_tree = expect_one_tree(tree, |child_tree| {
                matches!(child_tree.kind, TreeKind::ChangeGroupList)
            })?;
            let change_group_markdown_node =
                expect_one_markdown_node(change_group_list_tree, |child_tree| {
                    matches!(child_tree, Node::List(_))
                })?;

            if let TreeKind::ChangeGroupHeader(change_group) = &change_group_tree.kind {
                if let Node::List(list) = change_group_markdown_node {
                    for list_item in &list.children {
                        if let Node::ListItem(_) = list_item {
                            changes.push(
                                to_markdown(list_item)
                                    .expect("This should not fail")
                                    .trim_end()
                                    .trim_start_matches("* ") // remove the leading bullet '*'
                                    .to_string(),
                            );
                        }
                    }
                }
                results.push((change_group.clone(), changes));
            }
        }
    }
    Ok(results)
}

impl Display for Changelog {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "# {CHANGELOG_TITLE}")?;
        write!(f, "\n\n")?;
        write!(f, "{NOTABLE_CHANGES_TEXT}")?;
        write!(f, "\n\n")?;
        write!(f, "{ABOUT_FORMAT_TEXT}")?;
        write!(f, "\n\n")?;

        write!(f, "## [Unreleased]")?;
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

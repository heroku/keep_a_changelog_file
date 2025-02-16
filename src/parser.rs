use crate::{ChangeGroup, ReleaseDate, ReleaseLink, ReleaseTag, ReleaseVersion};
use indexmap::IndexMap;
use markdown::mdast::Node;
use markdown::unist::Position;
use markdown::{to_mdast, ParseOptions};
use regex_lite::Regex;
use std::cell::Cell;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::LazyLock;
use std::vec::IntoIter;

pub(crate) const CHANGELOG_TITLE: &str = "Changelog";

pub(crate) const NOTABLE_CHANGES_TEXT: &str =
    "All notable changes to this project will be documented in this file.";

pub(crate) const ABOUT_FORMAT_TEXT: &str = "\
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).";

pub(crate) const UNRELEASED_HEADER_TEXT: &str = "Unreleased";

static DEFAULT_POSITION: LazyLock<Position> = LazyLock::new(|| Position::new(1, 1, 0, 1, 1, 0));

// We're trying to parse the markdown into a structure kind of like this:
//
// ChangelogFile     -> Title NotableChanges AboutFormat Unreleased *Release *ReleaseLink
// Title             -> <Node::Header("# Changelog")>
// NotableChanges    -> <Node::Paragraph("{NOTABLE_CHANGES_TEXT}")>
// AboutFormat       -> <Node::Paragraph("{ABOUT_FORMAT_TEXT}")>
// Unreleased        -> UnreleasedHeader *ChangeGroup
// UnreleasedHeader  -> <Node::Header("## Unreleased")>
// Release           -> ReleaseHeader +ChangeGroup
// ReleaseHeader     -> ReleaseVersion "-" ReleaseDate ?ReleaseTag
// ReleaseVersion    -> +"[0-9]" "." +"[0-9]" "." +[0-9]
// ReleaseDate       -> 4*"[0-9]" "-" 2*"[0-9]" "-" 2*"[0-9]"
// ReleaseTag        -> "[YANKED]" | "[NO CHANGES]"
// ChangeGroup       -> ChangeGroupHeader ChangeGroupList
// ChangeGroupHeader -> "### Added" | "### Removed" | "### Changed" | "### Fixed" | "### Security" | "### Deprecated"
// ChangeGroupList   -> <Node::List>
// ReleaseLink       -> <Node::Definition>
//
// Into a tree like this:
// ChangelogFile
//   Title
//   NotableChanges
//   AboutFormat
//   Unreleased
//     UnreleasedHeader
//     ChangeGroup
//       ChangeGroupHeader
//       ChangeGroupList
//   Release
//     ReleaseHeader
//       ReleaseVersion
//       ReleaseDate
//       ReleaseTag
//     ChangeGroup
//       ChangeGroupHeader
//       ChangeGroupList
//    ReleaseLink
//    ReleaseLink
//
// While being resilient to invalid or badly formatted markdown.
pub(crate) fn parse(contents: &str) -> Tree {
    let mut parser = Parser::new(contents);
    changelog_file(&mut parser);
    parser.build_tree()
}

macro_rules! format_to {
    ($buf:expr) => ();
    ($buf:expr, $lit:literal $($arg:tt)*) => {
        { use ::std::fmt::Write as _; let _ = ::std::write!($buf, $lit $($arg)*); }
    };
}

#[derive(Eq, PartialEq)]
pub(crate) struct Tree {
    pub(crate) kind: TreeKind,
    pub(crate) children: Vec<Child>,
}

impl Tree {
    fn print(&self, buf: &mut String, level: usize) {
        let indent = "  ".repeat(level);
        format_to!(buf, "{indent}{:?}\n", self.kind);
        for tree in self.tree_children() {
            tree.print(buf, level + 1);
        }
        assert!(buf.ends_with('\n'));
    }

    pub(crate) fn get_diagnostics(&self) -> Vec<Diagnostic> {
        assert_eq!(self.kind, TreeKind::ChangelogFile);

        let mut errors = vec![];

        for tree in self.tree_iter() {
            if let TreeKind::Error(parser_error) = &tree.kind {
                errors.push(Diagnostic::new(parser_error.to_string(), tree.position()));
            } else if tree.kind == TreeKind::Release || tree.kind == TreeKind::Unreleased {
                validate_change_groups(tree, &mut errors);
            }
        }

        validate_release_links(self, &mut errors);

        errors
    }

    fn child_iter(&self) -> IntoIter<&Child> {
        fn append<'a>(child: &'a Child, result: &mut Vec<&'a Child>) {
            result.push(child);
            if let Child::Tree(tree) = child {
                for child in &tree.children {
                    append(child, result);
                }
            }
        }

        let mut result = vec![];
        for child in &self.children {
            append(child, &mut result);
        }
        result.into_iter()
    }

    fn tree_iter(&self) -> IntoIter<&Tree> {
        self.child_iter()
            .filter_map(|child| {
                if let Child::Tree(tree) = child {
                    Some(tree)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub(crate) fn tree_children(&self) -> IntoIter<&Tree> {
        self.children
            .iter()
            .filter_map(|child| {
                if let Child::Tree(tree) = child {
                    Some(tree)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn position(&self) -> Position {
        let mut positions = self.child_iter().filter_map(|item| match item {
            Child::Markdown(node) => node.position(),
            Child::Dummy(position) => Some(position),
            Child::Tree(_) => None,
        });
        let first = positions.next().unwrap_or(&DEFAULT_POSITION);
        let last = positions.last().unwrap_or(first);
        Position::new(
            first.start.line,
            first.start.column,
            first.start.offset,
            last.end.line,
            last.end.column,
            last.end.offset,
        )
    }
}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = String::new();
        self.print(&mut buf, 0);
        write!(f, "{buf}")
    }
}

fn validate_release_links(tree: &Tree, errors: &mut Vec<Diagnostic>) {
    let mut release_and_link_map: IndexMap<&ReleaseVersion, (Option<&Tree>, Option<&Tree>)> =
        IndexMap::new();

    for tree in tree.tree_iter() {
        match &tree.kind {
            TreeKind::ReleaseHeader(release_version, _, _) => {
                match release_and_link_map.get(release_version) {
                    Some((Some(_), _)) => {
                        errors.push(Diagnostic::new(
                            format!("Duplicate release version '{release_version}' found"),
                            tree.position().clone(),
                        ));
                    }
                    Some((None, maybe_link)) => {
                        release_and_link_map.insert(release_version, (Some(tree), *maybe_link));
                    }
                    None => {
                        release_and_link_map.insert(release_version, (Some(tree), None));
                    }
                }
            }
            TreeKind::ReleaseLink(ReleaseLinkType::Versioned(release_version, _)) => {
                match release_and_link_map.get(release_version) {
                    Some((_, Some(_))) => {
                        errors.push(Diagnostic::new(
                            format!("Duplicate release version link '{release_version}' found"),
                            tree.position().clone(),
                        ));
                    }
                    Some((maybe_release, None)) => {
                        release_and_link_map.insert(release_version, (*maybe_release, Some(tree)));
                    }
                    None => {
                        release_and_link_map.insert(release_version, (None, Some(tree)));
                    }
                }
            }
            _ => {
                // no validation needed
            }
        }
    }

    release_and_link_map.values().for_each(|entry| {
        if let (None, Some(release_link)) = entry {
            errors.push(Diagnostic::new(
                "Release link version does not match any listed releases".to_string(),
                release_link.position().clone(),
            ));
        }
    });
}

fn validate_change_groups(tree: &Tree, errors: &mut Vec<Diagnostic>) {
    let mut change_groups = HashSet::new();
    for tree in tree.tree_iter() {
        if let TreeKind::ChangeGroupHeader(change_group) = &tree.kind {
            if change_groups.contains(change_group) {
                errors.push(Diagnostic::new(
                    "Duplicate change group found".to_string(),
                    tree.position().clone(),
                ));
            } else {
                change_groups.insert(change_group);
            }
        }
    }

    if tree.kind == TreeKind::Release
        && change_groups.is_empty()
        && !tree.children.iter().any(|child| {
            matches!(
                child,
                Child::Tree(Tree {
                    kind: TreeKind::ReleaseHeader(_, _, Some(ReleaseTag::NoChanges)),
                    ..
                })
            )
        })
    {
        errors.push(Diagnostic::new(
            "Release must have at least one change group listed or be tagged with [NO CHANGES]"
                .to_string(),
            tree.position().clone(),
        ));
    }
}

/// Describes a problem or hint for a piece of the changelog document.
#[derive(Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// The message associated with this diagnostic.
    pub message: String,
    /// Location of the diagnostic in a changelog document.
    pub position: Position,
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{line}:{column} {message}",
            line = self.position.start.line,
            column = self.position.start.column,
            message = self.message
        )
    }
}

impl Diagnostic {
    fn new(message: String, position: Position) -> Self {
        Self { message, position }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ParserError(String);

impl Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum TreeKind {
    Error(ParserError),
    Title,
    ChangelogFile,
    NotableChanges,
    AboutFormat,
    Unreleased,
    UnreleasedHeader,
    Release,
    ReleaseHeader(ReleaseVersion, ReleaseDate, Option<ReleaseTag>),
    ChangeGroup,
    ChangeGroupHeader(ChangeGroup),
    ChangeGroupList,
    ReleaseLink(ReleaseLinkType),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Child {
    Markdown(Node),
    Tree(Tree),
    Dummy(Position),
}

// kind of like a text token but with nodes
enum ParserToken<'a> {
    Value(&'a Node),
    Eof,
}

#[derive(Debug)]
enum Event {
    Open { kind: TreeKind },
    Close,
    Advance,
    Missing,
}

struct MarkOpened {
    index: usize,
}

struct MarkClosed;

#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) enum ReleaseLinkType {
    Unreleased(ReleaseLink),
    Versioned(ReleaseVersion, ReleaseLink),
}

struct Parser {
    nodes: Vec<Node>,
    pos: usize,
    fuel: Cell<u32>,
    events: Vec<Event>,
    doc_length: usize,
}

impl Parser {
    fn new(contents: &str) -> Self {
        let nodes = to_mdast(contents, &ParseOptions::default())
            .ok()
            .map(|node| match node {
                Node::Root(root) => root.children,
                _ => vec![],
            })
            .unwrap_or_default();

        Self {
            nodes,
            pos: 0,
            fuel: Cell::new(256),
            events: vec![],
            doc_length: contents.len(),
        }
    }

    fn build_tree(self) -> Tree {
        let mut nodes = self.nodes.into_iter();
        let mut events = self.events;
        let mut previous_position = None;

        assert!(matches!(events.pop(), Some(Event::Close)));
        let mut stack = Vec::new();
        for event in events {
            match event {
                Event::Open { kind } => stack.push(Tree {
                    kind,
                    children: Vec::new(),
                }),
                Event::Close => {
                    let tree = stack.pop().expect("stack should not be empty");
                    stack
                        .last_mut()
                        .expect("a parent tree should always be present")
                        .children
                        .push(Child::Tree(tree));
                }
                Event::Advance => {
                    let node = nodes.next().expect(
                        "advance events should correspond to an existing node being present",
                    );
                    previous_position = node.position().cloned();
                    stack
                        .last_mut()
                        .expect("a parent tree should always be present")
                        .children
                        .push(Child::Markdown(node));
                }
                Event::Missing => {
                    stack
                        .last_mut()
                        .expect("a parent tree should always be present")
                        .children
                        .push(Child::Dummy(get_dummy_node_position(
                            previous_position.as_ref(),
                            self.doc_length,
                        )));
                }
            }
        }

        let tree = stack.pop().expect("stack should not be empty");

        // sanity check that we've consumed all nodes and events
        assert!(stack.is_empty());
        assert!(nodes.next().is_none());

        tree
    }

    fn open(&mut self) -> MarkOpened {
        let mark = MarkOpened {
            index: self.events.len(),
        };
        self.events.push(Event::Open {
            kind: TreeKind::Error(ParserError("Unclosed Event".to_string())),
        });
        mark
    }

    fn close(&mut self, m: &MarkOpened, kind: TreeKind) -> MarkClosed {
        self.events[m.index] = Event::Open { kind };
        self.events.push(Event::Close);
        MarkClosed {}
    }

    fn advance(&mut self) {
        assert!(!self.eof());
        self.fuel.set(256);
        self.events.push(Event::Advance);
        self.pos += 1;
    }

    fn advance_with_error(&mut self, error_fn: impl FnOnce(&Node) -> String) {
        assert!(!self.eof());
        let error_message = if let ParserToken::Value(node) = self.nth(0) {
            error_fn(node)
        } else {
            unreachable!("This should never happen since we already asserted we aren't at eof")
        };
        let m = self.open();
        self.advance();
        self.close(&m, TreeKind::Error(ParserError(error_message)));
    }

    fn capture_missing_node(&mut self, error: impl Into<String>) {
        let m = self.open();
        self.events.push(Event::Missing);
        self.close(&m, TreeKind::Error(ParserError(error.into())));
    }

    fn eof(&self) -> bool {
        self.pos == self.nodes.len()
    }

    fn nth(&self, lookahead: usize) -> ParserToken {
        assert_ne!(self.fuel.get(), 0, "parser is stuck");
        self.fuel.set(self.fuel.get() - 1);
        self.nodes
            .get(self.pos + lookahead)
            .map_or(ParserToken::Eof, ParserToken::Value)
    }

    fn at(&self, test_fn: fn(&Node) -> bool) -> bool {
        match self.nth(0) {
            ParserToken::Value(node) => test_fn(node),
            ParserToken::Eof => false,
        }
    }

    fn at_any(&self, test_fns: &[fn(&Node) -> bool]) -> bool {
        test_fns.iter().any(|f| self.at(*f))
    }

    fn eat(&mut self, test_fn: fn(&Node) -> bool) -> bool {
        if self.at(test_fn) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, test_fn: fn(&Node) -> bool, else_error: fn(&Node) -> String) {
        if !self.eat(test_fn) {
            if let ParserToken::Value(_) = self.nth(0) {
                self.advance_with_error(else_error);
            }
        }
    }
}

fn get_dummy_node_position(position_before: Option<&Position>, doc_length: usize) -> Position {
    if let Some(position) = position_before {
        Position::new(
            position.end.line + 1,
            1,
            (position.end.offset + 1).clamp(0, doc_length),
            position.end.line + 1,
            1,
            (position.end.offset + 1).clamp(0, doc_length),
        )
    } else {
        Position::new(1, 1, 0, 1, 1, 0)
    }
}

fn to_markdown(node: &Node) -> String {
    mdast_util_to_markdown::to_markdown(node).map_or_else(
        |message| format!("Error converting node to markdown - {message}"),
        |s| s.trim_end().to_string(),
    )
}

fn matches_markdown(node: &Node, markdown: &str) -> bool {
    mdast_util_to_markdown::to_markdown(node)
        .ok()
        .is_some_and(|s| {
            s.split_ascii_whitespace().collect::<Vec<_>>()
                == markdown.split_ascii_whitespace().collect::<Vec<_>>()
        })
}

fn changelog_file(p: &mut Parser) {
    let m = p.open();
    title(p);
    notable_changes_text(p);
    about_format_text(p);
    unreleased(p);
    while !p.eof() {
        if p.at(|node| matches!(node, Node::Heading(h) if h.depth == 2)) {
            release(p);
        } else if p.at(|node| matches!(node, Node::Definition(_))) {
            release_link(p);
        } else {
            p.advance_with_error(|node|
                format!("Unexpected markdown - Expected either a Release Header or Release Link here but found:\n\n{}", to_markdown(node))
            );
        }
    }
    p.close(&m, TreeKind::ChangelogFile);
}

fn title(p: &mut Parser) {
    if p.at(|node| matches!(node, Node::Heading(h) if h.depth == 1)) {
        let m = p.open();
        p.expect(
            |node| node.to_string() == CHANGELOG_TITLE,
            |node| {
                format!(
                    "Expected '# {CHANGELOG_TITLE}' but found '# {}'",
                    node.to_string()
                )
            },
        );
        p.close(&m, TreeKind::Title);
    } else {
        p.capture_missing_node(format!(
            "The following markdown is missing:\n\n# {CHANGELOG_TITLE}\n\nIt must appear at the start of the document."
        ));
    }
}

fn notable_changes_text(p: &mut Parser) {
    if p.at(|node| matches!(node, Node::Paragraph(_)))
        && !p.at(|node| matches_markdown(node, ABOUT_FORMAT_TEXT))
    {
        let m = p.open();
        p.expect(
            |node| node.to_string() == NOTABLE_CHANGES_TEXT,
            |node| {
                format!(
                    "Expected the following markdown:\n\n{NOTABLE_CHANGES_TEXT}\n\nbut was:\n\n{}",
                    to_markdown(node)
                )
            },
        );
        p.close(&m, TreeKind::NotableChanges);
    } else {
        p.capture_missing_node(format!(
            "The following markdown is missing:\n\n{NOTABLE_CHANGES_TEXT}\n\nIt must appear after:\n\n# {CHANGELOG_TITLE}"
        ));
    }
}

fn about_format_text(p: &mut Parser) {
    if p.at(|node| matches!(node, Node::Paragraph(_))) {
        let m = p.open();
        p.expect(
            |node| matches_markdown(node, ABOUT_FORMAT_TEXT),
            |node| {
                format!(
                    "Expected the following markdown:\n\n{ABOUT_FORMAT_TEXT}\n\nbut was:\n\n{}",
                    to_markdown(node)
                )
            },
        );
        p.close(&m, TreeKind::AboutFormat);
    } else {
        p.capture_missing_node(format!(
            "The following markdown is missing:\n\n{ABOUT_FORMAT_TEXT}\n\nIt must appear after:\n\n{NOTABLE_CHANGES_TEXT}"
        ));
    }
}

fn unreleased(p: &mut Parser) {
    if p.at(|node| matches!(node, Node::Heading(h) if h.depth == 2))
        && !p.at(|node| RELEASE_HEADER_REGEX.is_match(node.to_string().as_str()))
    {
        let m = p.open();
        {
            let m = p.open();
            p.expect(
                |node| UNRELEASED_HEADER_REGEX.is_match(node.to_string().as_str()),
                |node| {
                    format!(
                        "Expected '## {UNRELEASED_HEADER_TEXT}' but found '{}'",
                        to_markdown(node)
                    )
                },
            );
            p.close(&m, TreeKind::UnreleasedHeader);
        }

        while !p.eof() {
            if p.at(|node| matches!(node, Node::Heading(h) if h.depth == 3)) {
                change_group(p);
            } else if p.at_any(&[
                |node| matches!(node, Node::Heading(h) if h.depth == 2),
                |node| matches!(node, Node::Definition(_)),
            ]) {
                break;
            } else {
                p.advance_with_error(|node| format!("Unexpected markdown - '## {UNRELEASED_HEADER_TEXT}' should be followed by either a Change Group, Release, or Release Link but was:\n\n{}", to_markdown(node)));
            }
        }

        p.close(&m, TreeKind::Unreleased);
    } else {
        p.capture_missing_node(format!("The following markdown is missing:\n\n## {UNRELEASED_HEADER_TEXT}\n\nIt must appear after:\n\n{ABOUT_FORMAT_TEXT}"));
    }
}

static UNRELEASED_HEADER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\[Unreleased]|Unreleased)$").expect("Should be a valid regex"));

fn release(p: &mut Parser) {
    if p.at(|node| matches!(node, Node::Heading(h) if h.depth == 2)) {
        let m = p.open();

        release_header(p);

        while !p.eof() {
            if p.at(|node| matches!(node, Node::Heading(h) if h.depth == 3)) {
                change_group(p);
            } else if p.at_any(&[
                |node| matches!(node, Node::Heading(h) if h.depth == 2),
                |node| matches!(node, Node::Definition(_)),
            ]) {
                break;
            } else {
                p.advance_with_error(|node| format!("Unexpected markdown - Release should be followed by either a Change Group, Release, or Release Link but was:\n\n{}", to_markdown(node)));
            }
        }

        p.close(&m, TreeKind::Release);
    }
}

fn release_header(p: &mut Parser) {
    if let ParserToken::Value(node) = p.nth(0) {
        if let Some(captures) = RELEASE_HEADER_REGEX.captures(node.to_string().as_str()) {
            let release_version = match captures["version"].parse::<ReleaseVersion>() {
                Ok(v) => v,
                Err(e) => {
                    p.advance_with_error(|_| {
                        format!("Invalid release version '{}' - {e}", &captures["version"])
                    });
                    return;
                }
            };

            let release_date = match captures["release_date"].parse::<ReleaseDate>() {
                Ok(v) => v,
                Err(e) => {
                    p.advance_with_error(|_| {
                        format!("Invalid release date '{}' - {e}", &captures["release_date"])
                    });
                    return;
                }
            };

            let release_tag = if let Some(tag_value) = captures.name("tag") {
                match tag_value.as_str().parse::<ReleaseTag>() {
                    Ok(v) => Some(v),
                    Err(e) => {
                        p.advance_with_error(|_| {
                            format!("Invalid release tag '{}' - {e}", &captures["tag"])
                        });
                        return;
                    }
                }
            } else {
                None
            };

            let m = p.open();
            p.advance();
            p.close(
                &m,
                TreeKind::ReleaseHeader(release_version, release_date, release_tag),
            );
        } else {
            p.advance_with_error(|node| format!(
                "Expected Release Header with the format '[<semver>] - <YYYY>-<MM>-<DD> - [<tag>]' but found '{}'",
                to_markdown(node)
            ));
        }
    }
}

static RELEASE_HEADER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[?(?P<version>[^]\s-]+)]?\s+-\s+(?P<release_date>[^\s]+)(?:\s+\[(?P<tag>.+)])?$")
        .expect("Should be a valid regex")
});

fn change_group(p: &mut Parser) {
    if p.at(|node| matches!(node, Node::Heading(h) if h.depth == 3)) {
        let m = p.open();

        change_group_header(p);

        if p.at(|node| matches!(node, Node::List(_))) {
            let m = p.open();
            p.advance();
            p.close(&m, TreeKind::ChangeGroupList);
        } else {
            p.capture_missing_node("Change Group is missing the required list of changes");
        }

        p.close(&m, TreeKind::ChangeGroup);
    } else {
        p.capture_missing_node(format!(
            "Missing one of the following change groups: {}",
            format_change_group_headers()
        ));
    }
}

fn change_group_header(p: &mut Parser) {
    if let ParserToken::Value(node) = p.nth(0) {
        let Ok(change_group) = ChangeGroup::from_str(node.to_string().as_str()) else {
            p.advance_with_error(|node| {
                format!(
                    "Expected one of the following change groups:\n\n{}\n\nbut found:\n\n{}",
                    format_change_group_headers(),
                    to_markdown(node)
                )
            });
            return;
        };

        let m = p.open();
        p.advance();
        p.close(&m, TreeKind::ChangeGroupHeader(change_group));
    } else {
        p.advance_with_error(|node| {
            format!(
                "Expected one of the following change groups:\n\n{}\n\nbut found:\n\n{}",
                format_change_group_headers(),
                to_markdown(node)
            )
        });
    }
}

fn format_change_group_headers() -> String {
    [
        ChangeGroup::Added,
        ChangeGroup::Changed,
        ChangeGroup::Deprecated,
        ChangeGroup::Fixed,
        ChangeGroup::Removed,
        ChangeGroup::Security,
    ]
    .iter()
    .map(|v| format!("### {v}"))
    .collect::<Vec<_>>()
    .join(", ")
}

fn release_link(p: &mut Parser) {
    if let ParserToken::Value(Node::Definition(def)) = p.nth(0) {
        let identifier = def.identifier.clone();
        let url = def.url.clone();

        let release_link = match ReleaseLink::from_str(&url) {
            Ok(v) => v,
            Err(e) => {
                p.advance_with_error(|_| format!("Invalid url '{url}' in release link - {e}"));
                return;
            }
        };

        let release_link_version =
            if identifier.to_lowercase() == UNRELEASED_HEADER_TEXT.to_lowercase() {
                ReleaseLinkType::Unreleased(release_link)
            } else {
                match ReleaseVersion::from_str(&identifier) {
                    Ok(v) => ReleaseLinkType::Versioned(v, release_link),
                    Err(e) => {
                        p.advance_with_error(|_| {
                            format!("Invalid version '{identifier}' in release link - {e}")
                        });
                        return;
                    }
                }
            };

        let m = p.open();
        p.advance();
        p.close(&m, TreeKind::ReleaseLink(release_link_version));
    } else {
        p.advance_with_error(|node| {
            format!("Expected Release Link but found:\n\n{}", to_markdown(node))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::{formatdoc, indoc};

    #[test]
    fn test_empty_changelog_reports_all_required_information_as_missing() {
        let parsed_tree = parse("");
        assert_tree(
            &parsed_tree,
            ExpectTree::new(TreeKind::ChangelogFile, vec![
                expected_dummy_error(format!("The following markdown is missing:\n\n# {CHANGELOG_TITLE}\n\nIt must appear at the start of the document.")),
                expected_dummy_error(format!("The following markdown is missing:\n\n{NOTABLE_CHANGES_TEXT}\n\nIt must appear after:\n\n# {CHANGELOG_TITLE}")),
                expected_dummy_error(format!("The following markdown is missing:\n\n{ABOUT_FORMAT_TEXT}\n\nIt must appear after:\n\n{NOTABLE_CHANGES_TEXT}")),
                expected_dummy_error(format!("The following markdown is missing:\n\n## {UNRELEASED_HEADER_TEXT}\n\nIt must appear after:\n\n{ABOUT_FORMAT_TEXT}")),
            ]),
        );
    }

    #[test]
    fn test_changelog_title_typo() {
        let parsed_tree = parse(&formatdoc! { "
            # Chnglg
            
            {NOTABLE_CHANGES_TEXT}
            
            {ABOUT_FORMAT_TEXT}
            
            ## {UNRELEASED_HEADER_TEXT}
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    ExpectChild::tree(
                        TreeKind::Title,
                        vec![expected_markdown_error(
                            format!("Expected '# {CHANGELOG_TITLE}' but found '# Chnglg'"),
                            |node| matches!(node, Node::Heading(h) if h.depth == 1),
                        )],
                    ),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                ],
            ),
        );
    }

    #[test]
    fn test_changelog_with_missing_title() {
        let parsed_tree = parse(&formatdoc! { "
            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_dummy_error(
                        format!("The following markdown is missing:\n\n# {CHANGELOG_TITLE}\n\nIt must appear at the start of the document.")
                    ),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups()
                ],
            ),
        );
    }

    #[test]
    fn test_notable_changes_text_does_not_match() {
        let parsed_tree = parse(&formatdoc! { "
            # {CHANGELOG_TITLE}

            Not the text that should be here.
            
            {ABOUT_FORMAT_TEXT}
            
            ## {UNRELEASED_HEADER_TEXT}
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    ExpectChild::tree(TreeKind::NotableChanges, vec![
                        expected_markdown_error(
                            format!("Expected the following markdown:\n\n{NOTABLE_CHANGES_TEXT}\n\nbut was:\n\nNot the text that should be here."),
                            |node| matches!(node, Node::Paragraph(_)),
                        )
                    ]),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups()
                ],
            ),
        );
    }

    #[test]
    fn test_notable_changes_text_missing() {
        let parsed_tree = parse(&formatdoc! { "
            # {CHANGELOG_TITLE}

            {ABOUT_FORMAT_TEXT}
            
            ## {UNRELEASED_HEADER_TEXT}
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_dummy_error(
                        format!("The following markdown is missing:\n\n{NOTABLE_CHANGES_TEXT}\n\nIt must appear after:\n\n# {CHANGELOG_TITLE}"),
                    ),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups()
                ],
            ),
        );
    }

    #[test]
    fn test_about_format_text_does_not_match() {
        let parsed_tree = parse(&formatdoc! { "
            # {CHANGELOG_TITLE}
            
            {NOTABLE_CHANGES_TEXT}

            Not the text that should be here.
            
            ## {UNRELEASED_HEADER_TEXT}
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    ExpectChild::tree(TreeKind::AboutFormat, vec![
                        expected_markdown_error(
                            format!("Expected the following markdown:\n\n{ABOUT_FORMAT_TEXT}\n\nbut was:\n\nNot the text that should be here."),
                            |node| matches!(node, Node::Paragraph(_)),
                        )
                    ]),
                    expected_unreleased_with_no_change_groups()
                ],
            ),
        );
    }

    #[test]
    fn test_about_format_text_missing() {
        let parsed_tree = parse(&formatdoc! { "
            # {CHANGELOG_TITLE}
            
            {NOTABLE_CHANGES_TEXT}

            ## {UNRELEASED_HEADER_TEXT}
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_dummy_error(
                        format!("The following markdown is missing:\n\n{ABOUT_FORMAT_TEXT}\n\nIt must appear after:\n\n{NOTABLE_CHANGES_TEXT}"),
                    ),
                    expected_unreleased_with_no_change_groups()
                ],
            ),
        );
    }

    #[test]
    fn test_unreleased_header_does_not_match() {
        let parsed_tree = parse(&formatdoc! { "
            # {CHANGELOG_TITLE}
            
            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}
            
            ## Unrlsed
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    ExpectChild::tree(
                        TreeKind::Unreleased,
                        vec![ExpectChild::tree(
                            TreeKind::UnreleasedHeader,
                            vec![expected_markdown_error(
                                format!(
                                    "Expected '## {UNRELEASED_HEADER_TEXT}' but found '## Unrlsed'"
                                ),
                                |node| matches!(node, Node::Heading(h) if h.depth == 2),
                            )],
                        )],
                    ),
                ],
            ),
        );
    }

    #[test]
    fn test_unreleased_header_missing() {
        let parsed_tree = parse(&formatdoc! { "
            # {CHANGELOG_TITLE}
            
            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_dummy_error(format!("The following markdown is missing:\n\n## {UNRELEASED_HEADER_TEXT}\n\nIt must appear after:\n\n{ABOUT_FORMAT_TEXT}"))
                ],
            ),
        );
    }

    #[test]
    fn test_unreleased_change_group_header_missing() {
        let parsed_tree = parse(&formatdoc! { "
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            - test change
        "});
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_change_groups(vec![
                        expected_markdown_error(
                            format!("Unexpected markdown - '## {UNRELEASED_HEADER_TEXT}' should be followed by either a Change Group, Release, or Release Link but was:\n\n* test change"),
                            |node| matches!(node, Node::List(_)),
                        )
                    ]),
                ],
            ),
        );
    }

    #[test]
    fn test_unreleased_change_group_header_typo() {
        let parsed_tree = parse(&formatdoc! { "
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ### Chngd

            - test change
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_change_groups(vec![ExpectChild::tree(
                        TreeKind::ChangeGroup,
                        vec![
                            expected_markdown_error(
                                indoc! { "
                                    Expected one of the following change groups:
    
                                    ### Added, ### Changed, ### Deprecated, ### Fixed, ### Removed, ### Security
    
                                    but found:
    
                                    ### Chngd  
                                " }.trim(),
                                |node| matches!(node, Node::Heading(h) if h.depth == 3),
                            ),
                            expected_change_group_list(),
                        ],
                    )]),
                ],
            ),
        );
    }

    #[test]
    fn test_unreleased_change_group_header_recovery() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ### Removed

            ### Chngd

            - test change

            ### Fixed 

            - this should be good
        " });
        assert_tree(&parsed_tree, ExpectTree::new(
            TreeKind::ChangelogFile,
            vec![
                expected_title(),
                expected_notable_changes(),
                expected_about_format(),
                expected_unreleased_with_change_groups(vec![
                    ExpectChild::tree(
                        TreeKind::ChangeGroup,
                        vec![
                            expected_change_group_header(ChangeGroup::Removed),
                            expected_dummy_error("Change Group is missing the required list of changes"),
                        ],
                    ),
                    ExpectChild::tree(
                        TreeKind::ChangeGroup,
                        vec![
                            expected_markdown_error(
                                indoc! { "
                                        Expected one of the following change groups:
        
                                        ### Added, ### Changed, ### Deprecated, ### Fixed, ### Removed, ### Security
        
                                        but found:
        
                                        ### Chngd  
                                    " }.trim(),
                                |node| matches!(node, Node::Heading(h) if h.depth == 3),
                            ),
                            expected_change_group_list(),
                        ]
                    ),
                    expected_change_group(ChangeGroup::Fixed)
                ]),
            ]),
        );
    }

    #[test]
    fn test_unreleased_with_duplicate_change_group() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ### Removed

            - test change

            ### Fixed

            - test change

            ### Removed 

            - duplicate
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_change_groups(vec![
                        expected_change_group(ChangeGroup::Removed),
                        expected_change_group(ChangeGroup::Fixed),
                        expected_change_group(ChangeGroup::Removed),
                    ]),
                ],
            ),
        );

        let diagnostics = parsed_tree.get_diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "Unexpected number of diagnostics: {diagnostics:?}"
        );
        assert_eq!(diagnostics[0].message, "Duplicate change group found");
    }

    #[test]
    fn test_release_with_invalid_version() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ## [a.b.c] - 2000-01-01

            ### Fixed

            - test change
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                    ExpectChild::tree(TreeKind::Release, vec![
                        expected_markdown_error(
                            "Invalid release version 'a.b.c' - Could not parse version 'a.b.c' as semver.\nReason: unexpected character 'a' while parsing major version number",
                            |node| matches!(node, Node::Heading(h) if h.depth == 2),
                        ),
                        expected_change_group(ChangeGroup::Fixed)
                    ]),
                ],
            ),
        );
    }

    #[test]
    fn test_release_with_invalid_date() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ## [1.0.0] - 2000-99-99

            ### Fixed

            - test change
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                    ExpectChild::tree(TreeKind::Release, vec![
                        expected_markdown_error(
                            "Invalid release date '2000-99-99' - Could not parse release date '2000-99-99' as YYYY-MM-DD.\nReason: input is out of range",
                            |node| matches!(node, Node::Heading(h) if h.depth == 2),
                        ),
                        expected_change_group(ChangeGroup::Fixed)
                    ]),
                ],
            ),
        );
    }

    #[test]
    fn test_release_with_invalid_release_tag() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ## [1.0.0] - 2000-01-01 [YNKD]

            ### Fixed

            - test change
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                    ExpectChild::tree(TreeKind::Release, vec![
                        expected_markdown_error(
                            "Invalid release tag 'YNKD' - Could not parse release tag 'YNKD'\nExpected: YANKED | NO CHANGES",
                            |node| matches!(node, Node::Heading(h) if h.depth == 2),
                        ),
                        expected_change_group(ChangeGroup::Fixed)
                    ]),
                ],
            ),
        );
    }

    #[test]
    fn test_release_with_no_changes() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ## [1.0.0] - 2000-01-01
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                    expected_release(
                        "1.0.0".parse().unwrap(),
                        "2000-01-01".parse().unwrap(),
                        None,
                        vec![],
                    ),
                ],
            ),
        );

        let diagnostics = parsed_tree.get_diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "Unexpected number of diagnostics: {diagnostics:?}"
        );
        assert_eq!(
            diagnostics[0].message,
            "Release must have at least one change group listed or be tagged with [NO CHANGES]"
        );
    }

    #[test]
    fn test_release_change_header_typo() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ## bad header
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                    ExpectChild::tree(TreeKind::Release, vec![
                        expected_markdown_error(
                            "Expected Release Header with the format '[<semver>] - <YYYY>-<MM>-<DD> - [<tag>]' but found '## bad header'",
                            |node| matches!(node, Node::Heading(h) if h.depth == 2),
                        )
                    ]),
                ],
            ),
        );
    }

    #[test]
    fn test_release_change_header_recovery() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ## [1.0.0] - 2000-01-01

            ### Removed

            ### Chngd

            - test change

            ### Fixed 

            - this should be good
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                    expected_release("1.0.0".parse().unwrap(), "2000-01-01".parse().unwrap(), None, vec![
                        ExpectChild::tree(
                            TreeKind::ChangeGroup,
                            vec![
                                expected_change_group_header(ChangeGroup::Removed),
                                expected_dummy_error("Change Group is missing the required list of changes"),
                            ],
                        ),
                        ExpectChild::tree(
                            TreeKind::ChangeGroup,
                            vec![
                                expected_markdown_error(
                                    indoc! { "
                                        Expected one of the following change groups:
        
                                        ### Added, ### Changed, ### Deprecated, ### Fixed, ### Removed, ### Security
        
                                        but found:
        
                                        ### Chngd  
                                    " }.trim(),
                                    |node| matches!(node, Node::Heading(h) if h.depth == 3),
                                ),
                                expected_change_group_list(),
                            ]
                        ),
                        expected_change_group(ChangeGroup::Fixed)
                    ]),
                ],
            ),
        );
    }

    #[test]
    fn test_duplicate_release() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ## [1.0.0] - 2001-01-01

            ### Fixed

            - test change

            ## [1.0.0] - 2000-01-01

            ### Fixed

            - test change
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                    expected_release(
                        "1.0.0".parse().unwrap(),
                        "2001-01-01".parse().unwrap(),
                        None,
                        vec![expected_change_group(ChangeGroup::Fixed)],
                    ),
                    expected_release(
                        "1.0.0".parse().unwrap(),
                        "2000-01-01".parse().unwrap(),
                        None,
                        vec![expected_change_group(ChangeGroup::Fixed)],
                    ),
                ],
            ),
        );

        let diagnostics = parsed_tree.get_diagnostics();
        assert_eq!(
            diagnostics.len(),
            1,
            "Unexpected number of diagnostics: {diagnostics:?}"
        );
        assert_eq!(
            diagnostics[0].message,
            "Duplicate release version '1.0.0' found"
        );
    }

    #[test]
    fn test_release_with_duplicate_change_group() {
        let parsed_tree = parse(&formatdoc! {"
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}

            {ABOUT_FORMAT_TEXT}

            ## {UNRELEASED_HEADER_TEXT}

            ## [1.0.0] - 2000-01-01

            ### Fixed

            - test change

            ### Removed

            - test change

            ### Fixed 

            - duplicate
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                    expected_release(
                        "1.0.0".parse().unwrap(),
                        "2000-01-01".parse().unwrap(),
                        None,
                        vec![
                            expected_change_group(ChangeGroup::Fixed),
                            expected_change_group(ChangeGroup::Removed),
                            expected_change_group(ChangeGroup::Fixed),
                        ],
                    ),
                ],
            ),
        );

        let diagnostics = parsed_tree.get_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "Duplicate change group found");
    }

    #[test]
    fn test_release_link_validation() {
        let parsed_tree = parse(&formatdoc! { "
            # {CHANGELOG_TITLE}

            {NOTABLE_CHANGES_TEXT}
            
            {ABOUT_FORMAT_TEXT}
            
            ## {UNRELEASED_HEADER_TEXT}
            
            ## [2.0.0] - 2017-06-20
            
            ### Changed
            
            - test change
            
            ## [1.0.0] - 2017-06-20
            
            ### Changed
            
            - test change
            
            [2.0.0]: https://github.com/olivierlacan/keep-a-changelog/releases/tag/v2.0.0
            [2.0.0]: https://github.com/olivierlacan/keep-a-changelog/releases/tag/v2.0.0
            [0.0.1]: https://github.com/olivierlacan/keep-a-changelog/releases/tag/v0.0.1
        " });
        assert_tree(
            &parsed_tree,
            ExpectTree::new(
                TreeKind::ChangelogFile,
                vec![
                    expected_title(),
                    expected_notable_changes(),
                    expected_about_format(),
                    expected_unreleased_with_no_change_groups(),
                    expected_release(
                        "2.0.0".parse().unwrap(),
                        "2017-06-20".parse().unwrap(),
                        None,
                        vec![expected_change_group(ChangeGroup::Changed)],
                    ),
                    expected_release(
                        "1.0.0".parse().unwrap(),
                        "2017-06-20".parse().unwrap(),
                        None,
                        vec![expected_change_group(ChangeGroup::Changed)],
                    ),
                    expected_release_link(
                        "2.0.0".parse().unwrap(),
                        "https://github.com/olivierlacan/keep-a-changelog/releases/tag/v2.0.0"
                            .parse()
                            .unwrap(),
                    ),
                    expected_release_link(
                        "2.0.0".parse().unwrap(),
                        "https://github.com/olivierlacan/keep-a-changelog/releases/tag/v2.0.0"
                            .parse()
                            .unwrap(),
                    ),
                    expected_release_link(
                        "0.0.1".parse().unwrap(),
                        "https://github.com/olivierlacan/keep-a-changelog/releases/tag/v0.0.1"
                            .parse()
                            .unwrap(),
                    ),
                ],
            ),
        );

        let diagnostics = parsed_tree.get_diagnostics();
        assert_eq!(
            diagnostics.len(),
            2,
            "Unexpected number of diagnostics: {diagnostics:?}"
        );
        assert_eq!(
            diagnostics[0].message,
            "Duplicate release version link '2.0.0' found"
        );
        assert_eq!(
            diagnostics[1].message,
            "Release link version does not match any listed releases"
        );
    }

    #[derive(Debug)]
    struct ExpectTree {
        kind: TreeKind,
        children: Vec<ExpectChild>,
    }

    #[derive(Debug)]
    enum ExpectChild {
        Tree(ExpectTree),
        Markdown(fn(&Node) -> bool),
        Dummy,
    }

    impl ExpectChild {
        fn tree(kind: TreeKind, children: Vec<ExpectChild>) -> Self {
            ExpectChild::Tree(ExpectTree::new(kind, children))
        }
    }

    impl ExpectTree {
        fn new(kind: TreeKind, children: Vec<ExpectChild>) -> Self {
            ExpectTree { kind, children }
        }
    }

    fn assert_tree(parsed_tree: &Tree, expect_tree: ExpectTree) {
        assert_eq!(parsed_tree.kind, expect_tree.kind);
        assert!(
            expect_tree.children.len() >= parsed_tree.children.len(),
            "Missing expectations against children of: {parsed_tree:?}"
        );
        let mut parsed_children_iter = parsed_tree.children.iter();
        for expect_child in expect_tree.children {
            match (expect_child, parsed_children_iter.next()) {
                (ExpectChild::Tree(child_expect_tree), Some(Child::Tree(child_tree))) => {
                    assert_tree(child_tree, child_expect_tree);
                }
                (ExpectChild::Markdown(test_fn), Some(Child::Markdown(node))) => {
                    assert!(
                        test_fn(node),
                        "Expected markdown node to pass test function: {node:?}"
                    );
                }
                (ExpectChild::Dummy, Some(Child::Dummy(_))) => {
                    // we'll just skip these
                }
                (expect_child, Some(child)) => {
                    panic!("Different child type found:\nExpected - {expect_child:?}\nActual: {child:?}");
                }
                (expect_child, None) => {
                    panic!("No child node present:\nExpected - {expect_child:?}");
                }
            }
        }
    }

    fn expected_markdown_error(
        error_message: impl Into<String>,
        expected_error_node: fn(&Node) -> bool,
    ) -> ExpectChild {
        ExpectChild::tree(
            TreeKind::Error(ParserError(error_message.into())),
            vec![ExpectChild::Markdown(expected_error_node)],
        )
    }

    fn expected_dummy_error(error_message: impl Into<String>) -> ExpectChild {
        ExpectChild::tree(
            TreeKind::Error(ParserError(error_message.into())),
            vec![ExpectChild::Dummy],
        )
    }

    fn expected_title() -> ExpectChild {
        ExpectChild::tree(
            TreeKind::Title,
            vec![ExpectChild::Markdown(|node| {
                matches!(node, Node::Heading(d) if d.depth == 1)
                    && node.to_string() == CHANGELOG_TITLE
            })],
        )
    }

    fn expected_notable_changes() -> ExpectChild {
        ExpectChild::tree(
            TreeKind::NotableChanges,
            vec![ExpectChild::Markdown(|node| {
                matches!(node, Node::Paragraph(_)) && to_markdown(node) == NOTABLE_CHANGES_TEXT
            })],
        )
    }

    fn expected_about_format() -> ExpectChild {
        ExpectChild::tree(
            TreeKind::AboutFormat,
            vec![ExpectChild::Markdown(|node| {
                matches!(node, Node::Paragraph(_)) && to_markdown(node) == ABOUT_FORMAT_TEXT
            })],
        )
    }

    fn expected_unreleased_with_no_change_groups() -> ExpectChild {
        ExpectChild::tree(TreeKind::Unreleased, vec![expected_unreleased_header()])
    }

    fn expected_unreleased_with_change_groups(change_groups: Vec<ExpectChild>) -> ExpectChild {
        let mut expect_children = vec![expected_unreleased_header()];
        expect_children.extend(change_groups);
        ExpectChild::tree(TreeKind::Unreleased, expect_children)
    }

    fn expected_unreleased_header() -> ExpectChild {
        ExpectChild::tree(
            TreeKind::UnreleasedHeader,
            vec![ExpectChild::Markdown(|node| {
                matches!(node, Node::Heading(h) if h.depth == 2)
                    && UNRELEASED_HEADER_REGEX.is_match(node.to_string().as_str())
            })],
        )
    }

    fn expected_release(
        version: ReleaseVersion,
        date: ReleaseDate,
        tag: Option<ReleaseTag>,
        change_groups: Vec<ExpectChild>,
    ) -> ExpectChild {
        let mut expect_children = vec![expected_release_header(version, date, tag)];
        expect_children.extend(change_groups);
        ExpectChild::tree(TreeKind::Release, expect_children)
    }

    fn expected_release_header(
        version: ReleaseVersion,
        date: ReleaseDate,
        tag: Option<ReleaseTag>,
    ) -> ExpectChild {
        ExpectChild::tree(
            TreeKind::ReleaseHeader(version, date, tag),
            vec![ExpectChild::Markdown(
                |node| matches!(node, Node::Heading(h) if h.depth == 2),
            )],
        )
    }

    fn expected_change_group(change_group: ChangeGroup) -> ExpectChild {
        ExpectChild::tree(
            TreeKind::ChangeGroup,
            vec![
                expected_change_group_header(change_group),
                expected_change_group_list(),
            ],
        )
    }

    fn expected_change_group_header(change_group: ChangeGroup) -> ExpectChild {
        ExpectChild::tree(
            TreeKind::ChangeGroupHeader(change_group),
            vec![ExpectChild::Markdown(
                |node| matches!(node, Node::Heading(h) if h.depth == 3),
            )],
        )
    }

    fn expected_change_group_list() -> ExpectChild {
        ExpectChild::tree(
            TreeKind::ChangeGroupList,
            vec![ExpectChild::Markdown(|node| matches!(node, Node::List(_)))],
        )
    }

    fn expected_release_link(version: ReleaseVersion, url: ReleaseLink) -> ExpectChild {
        ExpectChild::tree(
            TreeKind::ReleaseLink(ReleaseLinkType::Versioned(version, url)),
            vec![ExpectChild::Markdown(|node| {
                matches!(node, Node::Definition(_))
            })],
        )
    }
}

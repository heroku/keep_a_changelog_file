use crate::ChangeGroup;
use markdown::mdast::{Node as MarkdownNode, Node};
use markdown::{to_mdast, ParseOptions};
use mdast_util_to_markdown::to_markdown;
use regex::Regex;
use std::cell::Cell;
use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

const CHANGELOG_TITLE: &str = "Changelog";

const NOTABLE_CHANGES_TEXT: &str =
    "All notable changes to this project will be documented in this file.";

const ABOUT_FORMAT_TEXT: &str = "\
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), \
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).";

const UNRELEASED_HEADER_TEXT: &str = "Unreleased";

fn parse(content: &str) -> Tree {
    let nodes = lex(content);
    let mut parser = Parser::new(nodes);
    changelog_file(&mut parser);
    parser.build_tree()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ChangelogNodeKind {
    Title,
    NotableChanges,
    AboutFormat,
    UnreleasedTitle,
    ReleaseTitle,
    ChangeGroupTitle,
    ChangeGroupList,
    ReleaseLink,
    ErrorNode,
    Eof,
}

#[derive(Debug)]
enum TreeKind {
    ErrorTree,
    ChangelogFile,
    Unreleased,
    Release,
    ChangeGroup,
}

#[derive(Debug)]
struct ChangelogNode {
    kind: ChangelogNodeKind,
    markdown: MarkdownNode,
}

struct Tree {
    kind: TreeKind,
    children: Vec<Child>,
}

enum Child {
    Node(ChangelogNode),
    Tree(Tree),
}

macro_rules! format_to {
    ($buf:expr) => ();
    ($buf:expr, $lit:literal $($arg:tt)*) => {
        { use ::std::fmt::Write as _; let _ = ::std::write!($buf, $lit $($arg)*); }
    };
}

impl Tree {
    fn print(&self, buf: &mut String, level: usize) {
        let indent = "  ".repeat(level);
        format_to!(buf, "{indent}{:?}\n", self.kind);
        for child in &self.children {
            match child {
                Child::Node(node) => {
                    let md = to_markdown(&node.markdown).unwrap_or("Error".to_string());
                    format_to!(buf, "{indent}  {:?} '{}'\n", node.kind, md.trim());
                }
                Child::Tree(tree) => tree.print(buf, level + 1),
            }
        }
        assert!(buf.ends_with('\n'));
    }
}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = String::new();
        self.print(&mut buf, 0);
        write!(f, "{buf}")
    }
}

fn lex(content: &str) -> Vec<ChangelogNode> {
    let markdown_nodes = to_mdast(content, &ParseOptions::default())
        .ok()
        .map(|node| match node {
            Node::Root(root) => root.children,
            _ => vec![],
        })
        .unwrap_or_default();
    let mut result = Vec::new();
    for markdown in markdown_nodes {
        let kind = 'kind: {
            if matches!(&markdown, MarkdownNode::Heading(h) if h.depth == 1 && markdown.to_string() == CHANGELOG_TITLE)
            {
                break 'kind ChangelogNodeKind::Title;
            }
            if matches!(&markdown, MarkdownNode::Heading(h) if h.depth == 2) {
                let header_text = markdown.to_string();
                if UNRELEASED_HEADER_TEXT == header_text
                    || format!("[{UNRELEASED_HEADER_TEXT}]") == header_text
                {
                    break 'kind ChangelogNodeKind::UnreleasedTitle;
                }
                if release_header_regex().is_match(markdown.to_string().as_str()) {
                    break 'kind ChangelogNodeKind::ReleaseTitle;
                }
                break 'kind ChangelogNodeKind::ErrorNode;
            }
            if matches!(&markdown, MarkdownNode::Heading(h) if h.depth == 3) {
                if ChangeGroup::from_str(&markdown.to_string()).is_ok() {
                    break 'kind ChangelogNodeKind::ChangeGroupTitle;
                }
                break 'kind ChangelogNodeKind::ErrorNode;
            }
            if matches!(&markdown, MarkdownNode::List(_)) {
                break 'kind ChangelogNodeKind::ChangeGroupList;
            }
            if matches!(&markdown, MarkdownNode::Paragraph(_)) {
                if markdown.to_string() == NOTABLE_CHANGES_TEXT {
                    break 'kind ChangelogNodeKind::NotableChanges;
                }
                if match_content(
                    to_markdown(&markdown).unwrap_or_default().as_str(),
                    ABOUT_FORMAT_TEXT,
                ) {
                    break 'kind ChangelogNodeKind::AboutFormat;
                }
                break 'kind ChangelogNodeKind::ErrorNode;
            }
            if matches!(markdown, MarkdownNode::Definition(_)) {
                break 'kind ChangelogNodeKind::ReleaseLink;
            }
            ChangelogNodeKind::ErrorNode
        };
        result.push(ChangelogNode { kind, markdown });
    }
    result
}

fn match_content(a: &str, b: &str) -> bool {
    a.split_ascii_whitespace().collect::<Vec<_>>() == b.split_ascii_whitespace().collect::<Vec<_>>()
}

static RELEASE_HEADER_REGEX: OnceLock<Regex> = OnceLock::new();

fn release_header_regex() -> &'static Regex {
    RELEASE_HEADER_REGEX.get_or_init(|| {
        let version = r"(?P<version>\d+\.\d+\.\d+)";
        let release_date = r"(?P<release_date>\d{4}-\d{2}-\d{2})";
        let tag = r"(?P<tag>.+)";
        Regex::new(&format!(
            r"^\[?{version}]?\s+-\s+{release_date}(?:\s+\[{tag}])?$"
        ))
        .expect("Should be a valid regex")
    })
}

#[derive(Debug)]
enum Event {
    Open { kind: TreeKind },
    Close,
    Advance,
}

struct MarkOpened {
    index: usize,
}

struct MarkClosed;

struct Parser {
    nodes: Vec<ChangelogNode>,
    pos: usize,
    fuel: Cell<u32>,
    events: Vec<Event>,
}

impl Parser {
    fn new(nodes: Vec<ChangelogNode>) -> Parser {
        Parser {
            nodes,
            pos: 0,
            fuel: Cell::new(256),
            events: Vec::new(),
        }
    }

    fn build_tree(self) -> Tree {
        let mut tokens = self.nodes.into_iter();
        let mut events = self.events;

        assert!(matches!(events.pop(), Some(Event::Close)));
        let mut stack = Vec::new();
        for event in events {
            match event {
                Event::Open { kind } => stack.push(Tree {
                    kind,
                    children: Vec::new(),
                }),
                Event::Close => {
                    let tree = stack.pop().unwrap();
                    stack.last_mut().unwrap().children.push(Child::Tree(tree));
                }
                Event::Advance => {
                    let token = tokens.next().unwrap();
                    stack.last_mut().unwrap().children.push(Child::Node(token));
                }
            }
        }

        let tree = stack.pop().unwrap();
        assert!(stack.is_empty());
        assert!(tokens.next().is_none());
        tree
    }

    fn open(&mut self) -> MarkOpened {
        let mark = MarkOpened {
            index: self.events.len(),
        };
        self.events.push(Event::Open {
            kind: TreeKind::ErrorTree,
        });
        mark
    }

    fn close(&mut self, m: MarkOpened, kind: TreeKind) -> MarkClosed {
        self.events[m.index] = Event::Open { kind };
        self.events.push(Event::Close);
        MarkClosed
    }

    fn advance(&mut self) {
        assert!(!self.eof());
        self.fuel.set(256);
        self.events.push(Event::Advance);
        self.pos += 1;
    }

    fn advance_with_error(&mut self, error: &str) {
        let m = self.open();
        // TODO: Error reporting.
        eprintln!("{error}");
        self.advance();
        self.close(m, TreeKind::ErrorTree);
    }

    fn eof(&self) -> bool {
        self.pos == self.nodes.len()
    }

    fn nth(&self, lookahead: usize) -> ChangelogNodeKind {
        assert_ne!(self.fuel.get(), 0, "parser is stuck");
        self.fuel.set(self.fuel.get() - 1);
        self.nodes
            .get(self.pos + lookahead)
            .map_or(ChangelogNodeKind::Eof, |it| it.kind)
    }

    fn at(&self, kind: ChangelogNodeKind) -> bool {
        self.nth(0) == kind
    }

    fn at_any(&self, kinds: &[ChangelogNodeKind]) -> bool {
        kinds.contains(&self.nth(0))
    }

    fn eat(&mut self, kind: ChangelogNodeKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: ChangelogNodeKind) {
        if self.eat(kind) {
            return;
        }
        // TODO: Error reporting.
        eprintln!("expected {kind:?}");
    }
}

fn changelog_file(p: &mut Parser) {
    let m = p.open();
    p.expect(ChangelogNodeKind::Title);
    p.expect(ChangelogNodeKind::NotableChanges);
    p.expect(ChangelogNodeKind::AboutFormat);
    unreleased(p);
    while p.at(ChangelogNodeKind::ReleaseTitle) && !p.eof() {
        release(p);
    }
    while !p.eof() {
        if p.at(ChangelogNodeKind::ReleaseLink) {
            p.advance();
        } else {
            p.advance_with_error("Expected release link");
        }
    }
    p.close(m, TreeKind::ChangelogFile);
}

fn unreleased(p: &mut Parser) {
    let m = p.open();
    p.expect(ChangelogNodeKind::UnreleasedTitle);
    while p.at(ChangelogNodeKind::ChangeGroupTitle) && !p.eof() {
        change_group(p);
    }
    p.close(m, TreeKind::Unreleased);
}

fn release(p: &mut Parser) {
    let m = p.open();
    p.expect(ChangelogNodeKind::ReleaseTitle);
    while p.at(ChangelogNodeKind::ChangeGroupTitle) && !p.eof() {
        change_group(p);
    }
    p.close(m, TreeKind::Release);
}

fn change_group(p: &mut Parser) {
    let m = p.open();
    p.expect(ChangelogNodeKind::ChangeGroupTitle);
    p.expect(ChangelogNodeKind::ChangeGroupList);
    p.close(m, TreeKind::ChangeGroup);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_full_changelog() {
        let cst = parse(KEEP_A_CHANGELOG);
        eprintln!("{cst:?}");
    }

    #[test]
    fn test_empty_changelog() {
        eprintln!("{:?}", parse(""));
    }

    #[test]
    fn test_changelog_with_just_a_title() {
        eprintln!("{:?}", parse("# Changelog"));
    }

    #[test]
    fn test_changelog_with_just_a_description() {
        eprintln!(
            "{:?}",
            parse(
                "
    All notable changes to this project will be documented in this file.
    
    The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
    and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
                "
            )
        );
    }

    #[test]
    fn test_changelog_with_just_an_unreleased_heading() {
        eprintln!("{:?}", parse("## Unreleased"));
    }

    #[test]
    fn test_changelog_with_just_a_title_and_unreleased_heading() {
        eprintln!("{:?}", parse("# Changelog\n\n## Unreleased"));
    }

    const KEEP_A_CHANGELOG: &str = r#"# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
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

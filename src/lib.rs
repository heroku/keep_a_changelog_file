#![doc = include_str!("../README.md")]

mod change_group;
mod changelog;
mod changes;
mod release;
mod release_date;
mod release_link;
mod release_tag;
mod release_version;
mod releases;
mod unreleased;

pub use crate::change_group::ChangeGroup;
pub use crate::changelog::Changelog;
pub use crate::changelog::ParseChangelogError;
pub use crate::changelog::PromoteOptions;
pub use crate::changelog::PromoteUnreleasedError;
pub use crate::changes::Changes;
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
use std::collections::HashMap;
use std::str::FromStr;

use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn changelog_js(config: String) -> String {
    let value: serde_json::Value = serde_json::from_str(&config).expect("deserialize json");
    let files = value
        .as_object()
        .expect("read as object")
        .get("files")
        .expect("get 'files'")
        .as_object()
        .expect("read files as object")
        .iter()
        .map(|(k, v)| {
            (
                k.to_string(),
                v.as_str().expect("read v as string").to_string(),
            )
        })
        .collect::<Vec<_>>();
    let mut results: HashMap<String, bool> = HashMap::new();
    for (file, contents) in files {
        match Changelog::from_str(&contents) {
            Ok(_) => results.insert(file, true),
            Err(_) => results.insert(file, false),
        };
    }
    serde_json::to_string(&results).expect("serialize to json")
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::changes::Changes;
    use crate::release::Release;
    use crate::unreleased::Unreleased;

    #[test]
    fn test_unreleased() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        assert_eq!(
            changelog.unreleased,
            Unreleased {
                link: Some(
                    "https://github.com/olivierlacan/keep-a-changelog/compare/v1.1.1...HEAD"
                        .parse()
                        .unwrap()
                ),
                ..Unreleased::default()
            }
        );
    }

    #[test]
    fn test_release_v1_1_1() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v1_1_1 = Release {
            version: "1.1.1".parse().unwrap(),
            date: "2023-03-05".parse().unwrap(),
            link: Some("https://github.com/olivierlacan/keep-a-changelog/compare/v1.1.0...v1.1.1"
                .parse()
                .unwrap()),
            tag: None,
            changes: Changes::from_iter([
                (
                    ChangeGroup::Added,
                    ["Arabic translation (#444).",
                        "v1.1 French translation.",
                        "v1.1 Dutch translation (#371).",
                        "v1.1 Russian translation (#410).",
                        "v1.1 Japanese translation (#363).",
                        "v1.1 Norwegian Bokmål translation (#383).",
                        "v1.1 \"Inconsistent Changes\" Turkish translation (#347).",
                        "Default to most recent versions available for each languages",
                        "Display count of available translations (26 to date!)",
                        "Centralize all links into `/data/links.json` so they can be updated easily",]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Fixed,
                    [
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
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Changed,
                    [
                        "Upgrade dependencies: Ruby 3.2.1, Middleman, etc."
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Removed,
                    [
                        "Unused normalize.css file",
                        "Identical links assigned in each translation file",
                        "Duplicate index file for the english version"
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
            ])
        };

        assert_eq!(
            changelog
                .releases
                .get_version(&"1.1.1".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v1_1_1
        );
    }

    #[test]
    fn test_release_v1_1_0() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v1_1_0 = Release {
            version: "1.1.0".parse().unwrap(),
            date: "2019-02-15".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v1.0.0...v1.1.0"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([
                (
                    ChangeGroup::Added,
                    [
                        "Danish translation (#297).",
                        "Georgian translation from (#337).",
                        "Changelog inconsistency section in Bad Practices.",
                    ]
                    .map(ToString::to_string)
                    .to_vec(),
                ),
                (
                    ChangeGroup::Fixed,
                    [
                        "Italian translation (#332).",
                        "Indonesian translation (#336).",
                    ]
                    .map(ToString::to_string)
                    .to_vec(),
                ),
            ]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"1.1.0".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v1_1_0
        );
    }

    #[test]
    fn test_release_v1_0_0() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v1_0_0 = Release {
            version: "1.0.0".parse().unwrap(),
            date: "2017-06-20".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.3.0...v1.0.0"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([
                (
                    ChangeGroup::Added,
                    [
                        "New visual identity by [@tylerfortune8](https://github.com/tylerfortune8).",
                        "Version navigation.",
                        "Links to latest released version in previous versions.",
                        "\"Why keep a changelog?\" section.",
                        "\"Who needs a changelog?\" section.",
                        "\"How do I make a changelog?\" section.",
                        "\"Frequently Asked Questions\" section.",
                        "New \"Guiding Principles\" sub-section to \"How do I make a changelog?\".",
                        "Simplified and Traditional Chinese translations from [@tianshuo](https://github.com/tianshuo).",
                        "German translation from [@mpbzh](https://github.com/mpbzh) & [@Art4](https://github.com/Art4).",
                        "Italian translation from [@azkidenz](https://github.com/azkidenz).",
                        "Swedish translation from [@magol](https://github.com/magol).",
                        "Turkish translation from [@emreerkan](https://github.com/emreerkan).",
                        "French translation from [@zapashcanon](https://github.com/zapashcanon).",
                        "Brazilian Portuguese translation from [@Webysther](https://github.com/Webysther).",
                        "Polish translation from [@amielucha](https://github.com/amielucha) & [@m-aciek](https://github.com/m-aciek).",
                        "Russian translation from [@aishek](https://github.com/aishek).",
                        "Czech translation from [@h4vry](https://github.com/h4vry).",
                        "Slovak translation from [@jkostolansky](https://github.com/jkostolansky).",
                        "Korean translation from [@pierceh89](https://github.com/pierceh89).",
                        "Croatian translation from [@porx](https://github.com/porx).",
                        "Persian translation from [@Hameds](https://github.com/Hameds).",
                        "Ukrainian translation from [@osadchyi-s](https://github.com/osadchyi-s).",
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Changed,
                    [
                        "Start using \"changelog\" over \"change log\" since it's the common usage.",
                        "Start versioning based on the current English version at 0.3.0 to help\n  translation authors keep things up-to-date.",
                        "Rewrite \"What makes unicorns cry?\" section.",
                        "Rewrite \"Ignoring Deprecations\" sub-section to clarify the ideal\n  scenario.",
                        "Improve \"Commit log diffs\" sub-section to further argument against\n  them.",
                        "Merge \"Why can’t people just use a git log diff?\" with \"Commit log\n    diffs\".",
                        "Fix typos in Simplified Chinese and Traditional Chinese translations.",
                        "Fix typos in Brazilian Portuguese translation.",
                        "Fix typos in Turkish translation.",
                        "Fix typos in Czech translation.",
                        "Fix typos in Swedish translation.",
                        "Improve phrasing in French translation.",
                        "Fix phrasing and spelling in German translation.",
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Removed,
                    [
                        "Section about \"changelog\" vs \"CHANGELOG\"."
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
            ]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"1.0.0".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v1_0_0
        );
    }

    #[test]
    fn test_release_v0_3_0() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_3_0 = Release {
            version: "0.3.0".parse().unwrap(),
            date: "2015-12-03".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.2.0...v0.3.0"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([(
                ChangeGroup::Added,
                [
                    "RU translation from [@aishek](https://github.com/aishek).",
                    "pt-BR translation from [@tallesl](https://github.com/tallesl).",
                    "es-ES translation from [@ZeliosAriex](https://github.com/ZeliosAriex).",
                ]
                .map(ToString::to_string)
                .to_vec(),
            )]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.3.0".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_3_0
        );
    }

    #[test]
    fn test_release_v0_2_0() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_2_0 = Release {
            version: "0.2.0".parse().unwrap(),
            date: "2015-10-06".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.1.0...v0.2.0"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([
                (
                    ChangeGroup::Changed,
                    [
                        "Remove exclusionary mentions of \"open source\" since this project can\n  benefit both \"open\" and \"closed\" source projects equally.",
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
            ]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.2.0".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_2_0
        );
    }

    #[test]
    fn test_release_v0_1_0() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_1_0 = Release {
            version: "0.1.0".parse().unwrap(),
            date: "2015-10-06".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.8...v0.1.0"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([
                (
                    ChangeGroup::Added,
                    ["Answer \"Should you ever rewrite a change log?\"."]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Changed,
                    [
                        "Improve argument against commit logs.",
                        "Start following [SemVer](https://semver.org) properly.",
                    ]
                    .map(ToString::to_string)
                    .to_vec(),
                ),
            ]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.1.0".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_1_0
        );
    }

    #[test]
    fn test_release_v0_0_8() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_0_8 = Release {
            version: "0.0.8".parse().unwrap(),
            date: "2015-02-17".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.7...v0.0.8"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([
                (
                    ChangeGroup::Changed,
                    [
                        "Update year to match in every README example.",
                        "Reluctantly stop making fun of Brits only, since most of the world\n  writes dates in a strange way."
                    ]
                    .map(ToString::to_string)
                    .to_vec(),
                ),
                (
                    ChangeGroup::Fixed,
                    [
                        "Fix typos in recent README changes.",
                        "Update outdated unreleased diff link."
                    ]
                    .map(ToString::to_string)
                    .to_vec(),
                ),
            ]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.0.8".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_0_8
        );
    }

    #[test]
    fn test_release_v0_0_7() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_0_7 = Release {
            version: "0.0.7".parse().unwrap(),
            date: "2015-02-16".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.6...v0.0.7"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([
                (
                    ChangeGroup::Added,
                    ["Link, and make it obvious that date format is ISO 8601."]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Changed,
                    ["Clarified the section on \"Is there a standard change log format?\"."]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Fixed,
                    ["Fix Markdown links to tag comparison URL with footnote-style links."]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
            ]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.0.7".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_0_7
        );
    }

    #[test]
    fn test_release_v0_0_6() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_0_6 = Release {
            version: "0.0.6".parse().unwrap(),
            date: "2014-12-12".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.5...v0.0.6"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([(
                ChangeGroup::Added,
                ["README section on \"yanked\" releases."]
                    .map(ToString::to_string)
                    .to_vec(),
            )]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.0.6".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_0_6
        );
    }

    #[test]
    fn test_release_v0_0_5() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_0_5 = Release {
            version: "0.0.5".parse().unwrap(),
            date: "2014-08-09".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.4...v0.0.5"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([(
                ChangeGroup::Added,
                [
                    "Markdown links to version tags on release headings.",
                    "Unreleased section to gather unreleased changes and encourage note\n  keeping prior to releases."
                ]
                .map(ToString::to_string)
                .to_vec(),
            )]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.0.5".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_0_5
        );
    }

    #[test]
    fn test_release_v0_0_4() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_0_4 = Release {
            version: "0.0.4".parse().unwrap(),
            date: "2014-08-09".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.3...v0.0.4"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([
                (
                    ChangeGroup::Added,
                    [
                        "Better explanation of the difference between the file (\"CHANGELOG\")\n  and its function \"the change log\".",
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Changed,
                    [
                        "Refer to a \"change log\" instead of a \"CHANGELOG\" throughout the site\n  to differentiate between the file and the purpose of the file — the\n  logging of changes."
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
                (
                    ChangeGroup::Removed,
                    [
                        "Remove empty sections from CHANGELOG, they occupy too much space and\n  create too much noise in the file. People will have to assume that the\n  missing sections were intentionally left out because they contained no\n  notable changes."
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                )
            ]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.0.4".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_0_4
        );
    }

    #[test]
    fn test_release_v0_0_3() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_0_3 = Release {
            version: "0.0.3".parse().unwrap(),
            date: "2014-08-09".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.2...v0.0.3"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([(
                ChangeGroup::Added,
                ["\"Why should I care?\" section mentioning The Changelog podcast."]
                    .map(ToString::to_string)
                    .to_vec(),
            )]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.0.3".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_0_3
        );
    }

    #[test]
    fn test_release_v0_0_2() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_0_2 = Release {
            version: "0.0.2".parse().unwrap(),
            date: "2014-07-10".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/compare/v0.0.1...v0.0.2"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([(
                ChangeGroup::Added,
                ["Explanation of the recommended reverse chronological release ordering."]
                    .map(ToString::to_string)
                    .to_vec(),
            )]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.0.2".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_0_2
        );
    }

    #[test]
    fn test_release_v0_0_1() {
        let changelog: Changelog = KEEP_A_CHANGELOG.parse().unwrap();
        let release_v0_0_1 = Release {
            version: "0.0.1".parse().unwrap(),
            date: "2014-05-31".parse().unwrap(),
            link: Some(
                "https://github.com/olivierlacan/keep-a-changelog/releases/tag/v0.0.1"
                    .parse()
                    .unwrap(),
            ),
            tag: None,
            changes: Changes::from_iter([
                (
                    ChangeGroup::Added,
                    [
                        "This CHANGELOG file to hopefully serve as an evolving example of a\n  standardized open source project CHANGELOG.",
                        "CNAME file to enable GitHub Pages custom domain.",
                        "README now contains answers to common questions about CHANGELOGs.",
                        "Good examples and basic guidelines, including proper date formatting.",
                        "Counter-examples: \"What makes unicorns cry?\"."
                    ]
                        .map(ToString::to_string)
                        .to_vec(),
                ),
            ]),
        };
        assert_eq!(
            changelog
                .releases
                .get_version(&"0.0.1".parse::<ReleaseVersion>().unwrap())
                .unwrap(),
            &release_v0_0_1
        );
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

#![allow(clippy::unwrap_used)]
#![allow(unused_crate_dependencies)]

use keep_a_changelog::{ChangeGroup, Changelog, PromoteOptions};

#[test]
fn adding_unreleased_changes() {
    let mut changelog: Changelog = "\
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]"
        .parse()
        .unwrap();

    changelog.unreleased.add(
        ChangeGroup::Fixed,
        "Fixed bug in feature X that would cause the machine to halt and catch fire.",
    );
    changelog.unreleased.add(
        ChangeGroup::Deprecated,
        "Feature Y will be removed from the next major release.",
    );

    assert_eq!(
        changelog.to_string(),
        format!(
            "\
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fixed bug in feature X that would cause the machine to halt and catch fire.

### Deprecated

- Feature Y will be removed from the next major release.\n"
        )
    );
}

#[test]
fn promoting_unreleased_changes() {
    let mut changelog: Changelog = "\
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fixed bug in feature X that would cause the machine to halt and catch fire.

### Deprecated

- Feature Y will be removed from the next major release.\n"
        .parse()
        .unwrap();

    let promote_options = PromoteOptions::new("0.0.1".parse().unwrap())
        .with_date("2023-01-01".parse().unwrap())
        .with_link(
            "https://github.com/my-org/my-project/releases/v0.0.1"
                .parse()
                .unwrap(),
        );

    changelog.promote_unreleased(&promote_options).unwrap();

    assert_eq!(
        changelog.to_string(),
        format!(
            "\
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2023-01-01

### Fixed

- Fixed bug in feature X that would cause the machine to halt and catch fire.

### Deprecated

- Feature Y will be removed from the next major release.

[0.0.1]: https://github.com/my-org/my-project/releases/v0.0.1\n"
        )
    );
}

#[test]
fn promoting_unreleased_to_existing_version() {
    let mut changelog: Changelog = "\
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added feature X

## [0.0.1] - 2023-01-01

### Fixed

- Fixed feature Y\n"
        .parse()
        .unwrap();

    let promote_options = PromoteOptions::new("0.0.1".parse().unwrap())
        .with_date("2023-01-01".parse().unwrap())
        .with_link(
            "https://github.com/my-org/my-project/releases/v0.0.1"
                .parse()
                .unwrap(),
        );

    assert!(changelog.promote_unreleased(&promote_options).is_err());
}

#[test]
fn parse_bad_changelog() {
    let changelog = "\
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [a.b.c] - Jan 1, 2023

- Fixed feature Y\n";

    assert!(changelog.parse::<Changelog>().is_err());
}

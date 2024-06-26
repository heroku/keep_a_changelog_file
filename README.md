# Keep a Changelog File &emsp; [![Build Status]][ci] [![Docs]][docs.rs] [![Latest Version]][crates.io] [![MSRV]][install-rust]

A serializer and deserializer for changelog files written in [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
format.

## Install

```sh
cargo add keep_a_changelog_file
```

## Usage

```rust
use keep_a_changelog_file::{
    Changelog,
    ChangeGroup,
    PromoteOptions,
    ReleaseLink,
    ReleaseVersion
};

fn example_usage() {
    // parse a changelog
    let mut changelog: Changelog = "\
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]"
        .parse()
        .unwrap();

    // modify the unreleased section
    changelog
        .unreleased
        .add(ChangeGroup::Fixed, "Fixed bug in feature X");
    changelog.unreleased.add(
        ChangeGroup::Deprecated,
        "Feature Y will be removed from the next major release",
    );

    // promote the unreleased changes to a new release
    let release_version = "0.0.1".parse::<ReleaseVersion>().unwrap();
    let release_link = "https://github.com/my-org/my-project/releases/v0.0.1"
        .parse::<ReleaseLink>()
        .unwrap();
    changelog
        .promote_unreleased(&PromoteOptions::new(release_version).with_link(release_link))
        .unwrap();

    // output the changelog
    println!("{changelog}");
}
```

[Build Status]: https://img.shields.io/github/actions/workflow/status/heroku/keep_a_changelog_file/ci.yml?branch=main

[ci]: https://github.com/heroku/keep_a_changelog_file/actions/workflows/ci.yml?query=branch%3Amain

[MSRV]: https://img.shields.io/badge/MSRV-rustc_1.74+-lightgray.svg

[install-rust]: https://www.rust-lang.org/tools/install

[Docs]: https://img.shields.io/docsrs/keep_a_changelog_file

[docs.rs]: https://docs.rs/keep_a_changelog_file/latest/keep_a_changelog_file/

[Latest Version]: https://img.shields.io/crates/v/keep_a_changelog_file.svg

[crates.io]: https://crates.io/crates/keep_a_changelog_file

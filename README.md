# Keep a Changelog Serializer/Deserializer

[![Build Status]][ci] [![MSRV]][install-rust]

[Build Status]: https://img.shields.io/github/actions/workflow/status/heroku/keep_a_changelog/ci.yml?branch=main
[ci]: https://github.com/heroku/keep_a_changelog/actions/workflows/ci.yml?query=branch%3Amain
[MSRV]: https://img.shields.io/badge/MSRV-rustc_1.74+-lightgray.svg
[install-rust]: https://www.rust-lang.org/tools/install

A serializer and deserializer for changelogs written in [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

## Install

```sh
cargo add --git https://github.com/heroku/keep_a_changelog
```

## Usage

```rust
use keep_a_changelog::{Changelog, ChangeGroup, PromoteOptions};

# fn main() {
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
    changelog.unreleased.add(
        ChangeGroup::Fixed, 
        "Fixed bug in feature X"
    );
    changelog.unreleased.add(
        ChangeGroup::Deprecated, 
        "Feature Y will be removed from the next major release"
    );
    
    // promote the unreleased changes to a new release
    let promote_options = PromoteOptions::new("0.0.1".parse().unwrap())
        .with_link("https://github.com/my-org/my-project/releases/v0.0.1".parse().unwrap());
    changelog.promote_unreleased(&promote_options).unwrap();

    // output the changelog
    println!("{}", changelog);
# }
```

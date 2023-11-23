# Keep a Changelog Serializer/Deserializer

[![Build Status]][ci] [![Docs]][docs.rs] [![Latest Version]][crates.io] [![MSRV]][install-rust]

[Build Status]: https://img.shields.io/github/actions/workflow/status/heroku/libcnb.rs/ci.yml?branch=main
[ci]: https://github.com/heroku/libcnb.rs/actions/workflows/ci.yml?query=branch%3Amain
[Docs]: https://img.shields.io/docsrs/libcnb
[docs.rs]: https://docs.rs/libcnb/latest/libcnb/
[Latest Version]: https://img.shields.io/crates/v/libcnb.svg
[crates.io]: https://crates.io/crates/libcnb
[MSRV]: https://img.shields.io/badge/MSRV-rustc_1.74+-lightgray.svg
[install-rust]: https://www.rust-lang.org/tools/install

A serializer and deserializer for changelogs written in [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

## Install

```sh
cargo add keep_a_changelog
```

## Usage

```rust
use keep_a_changelog::{Changelog, ChangeGroup, PromoteOptions};

fn main() {
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
}
```

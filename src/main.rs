#![allow(missing_docs)]
#![allow(unused_crate_dependencies)]
use std::fs;
use std::path::Path;

fn main() {
    println!("Hello, world!");
    println!("{:?}", std::env::args());
    let args = std::env::args().collect::<Vec<String>>();
    match args.first().map(String::as_str) {
        Some("validate") => {
            validate(&args[1..]);
        }
        Some(invalid_command) => {
            panic!("Not a valid command: {invalid_command}");
        }
        _ => {
            panic!("command argument is missing");
        }
    }
}

fn validate(args: &[String]) {
    let changelog = Path::new(args.first().map_or("CHANGELOG.md", |v| v.as_str()));
    if changelog.exists() {
        match fs::read_to_string(changelog) {
            Ok(content) => {
                println!("{content}");
            }
            _ => {
                println!("changelog could not be read: {}", changelog.display());
            }
        }
    } else {
        println!("changelog does not exist: {}", changelog.display());
    }
}

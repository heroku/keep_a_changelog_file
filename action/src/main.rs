mod gha;

use crate::gha::{get_multiline_input, InputError};
use glob::{glob, PatternError};
use keep_a_changelog_file::Changelog;
use std::fs;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn main() {
    println!("Hello, world!");
    if let Err(error) = execute_action() {
        todo!()
    }
}

fn execute_action() -> Result<(), ActionError> {
    let changelog_file_globs = get_multiline_input("changelog_files")
        .call()
        .map_err(ActionError::Input)?;

    for changelog_file_glob in changelog_file_globs {
        for entry in glob(&changelog_file_glob)
            .map_err(ActionError::Pattern)?
            .filter_map(Result::ok)
        {
            if entry.is_file() {
                validate_changelog(&entry)?;
            }
        }
    }

    Ok(())
}

fn validate_changelog(changelog_file: &Path) -> Result<(), ActionError> {
    let contents = fs::read_to_string(changelog_file)
        .map_err(|e| ActionError::ReadChangelog(changelog_file.to_path_buf(), e))?;

    match Changelog::from_str(&contents) {
        Ok(changelog) => {}
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                gha::error(&diagnostic.message)
                    .title(&diagnostic.message)
                    .file(changelog_file.to_path_buf())
                    .start_line(diagnostic.position.start.line)
                    .end_line(diagnostic.position.end.line)
                    .start_column(diagnostic.position.start.column)
                    .end_column(diagnostic.position.end.column)
                    .call();
            }
        }
    }

    Ok(())
}

enum ActionError {
    Input(InputError),
    Pattern(PatternError),
    ReadChangelog(PathBuf, std::io::Error),
}

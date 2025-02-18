mod gha;

use crate::gha::{get_multiline_input, InputError};
use glob::{glob, PatternError};
use keep_a_changelog_file::{Changelog, Diagnostic};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

fn main() {
    if let Err(error) = execute_action() {
        println!("{error:?}");
        std::process::exit(1);
    }
}

fn execute_action() -> Result<(), ActionError> {
    let changelog_file_globs = get_multiline_input("changelog_files")
        .trim_whitespace(true)
        .call()
        .map_err(ActionError::Input)?;

    let mut results = vec![];

    for changelog_file_glob in changelog_file_globs {
        for changelog_file in glob(&changelog_file_glob)
            .map_err(ActionError::Pattern)?
            .filter_map(Result::ok)
        {
            if changelog_file.is_file() {
                let contents = fs::read_to_string(&changelog_file)
                    .map_err(|e| ActionError::ReadChangelog(changelog_file.to_path_buf(), e))?;

                let diagnostics = match Changelog::from_str(&contents) {
                    Ok(_) => vec![],
                    Err(diagnostics) => diagnostics,
                };

                results.push((changelog_file, diagnostics));
            }
        }
    }

    print_summary_report(&results)?;

    let valid_changelogs = results
        .iter()
        .all(|(_, diagnostics)| diagnostics.is_empty());

    if !valid_changelogs {
        // TODO: raise validation error
    }

    Ok(())
}

macro_rules! format_to {
    ($buf:expr) => ();
    ($buf:expr, $lit:literal $($arg:tt)*) => {
        { use ::std::fmt::Write as _; let _ = ::std::write!($buf, $lit $($arg)*); }
    };
}

fn print_summary_report(results: &[(PathBuf, Vec<Diagnostic>)]) -> Result<(), ActionError> {
    let mut report = String::new();
    format_to!(report, "# Changelog Validation Report\n\n");
    for (changelog, diagnostics) in results {
        format_to!(report, "### `{}`\n\n", changelog.display());
        if diagnostics.is_empty() {
            format_to!(report, ":white_check_mark: Valid\n\n");
        } else {
            format_to!(report, ":x: Invalid\n\n");
            format_to!(report, "| Error | Line:Column |\n");
            format_to!(report, "|-------|-------------|\n");
            for diagnostic in diagnostics {
                format_to!(
                    report,
                    "| {message} | {line}:{column} |\n",
                    message = diagnostic.message,
                    line = diagnostic.position.start.line,
                    column = diagnostic.position.start.column
                );
            }
        }
    }

    let summary_file = std::env::var("GITHUB_STEP_SUMMARY")
        .map(PathBuf::from)
        .map_err(|_| ActionError::Environment("GITHUB_STEP_SUMMARY".to_string()))?;

    fs::write(&summary_file, report)
        .map_err(|e| ActionError::WriteSummaryReport(summary_file, e))?;

    Ok(())
}

#[derive(Debug)]
enum ActionError {
    Input(InputError),
    Pattern(PatternError),
    ReadChangelog(PathBuf, std::io::Error),
    Environment(String),
    WriteSummaryReport(PathBuf, std::io::Error),
}

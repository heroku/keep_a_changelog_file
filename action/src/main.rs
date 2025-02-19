use crate::gha::{get_boolean_input, get_multiline_input, github_step_summary, InputError};
use fun_run::CommandWithName;
use glob::{glob, PatternError};
use keep_a_changelog_file::{Changelog, Diagnostic};
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

mod gha;

fn main() {
    if let Err(error) = execute_action() {
        match error {
            ActionError::Input(error) => match error {
                InputError::MissingRequiredValue(name) => {
                    gha::error(format!("Missing required input `{name}`")).call();
                }
                InputError::InvalidBooleanType(name) => {
                    gha::error([
                        &format!("Input does not meet YAML 1.2 \"Core Schema\" specification: {name}"),
                        "Support boolean input list: `true | True | TRUE | false | False | FALSE`"
                    ].join("\n")).call();
                }
            },
            ActionError::Pattern(error) => {
                gha::error(format!(
                    "Invalid glob pattern in `changelog_files` input: `{error}`"
                ))
                .call();
            }
            ActionError::ReadChangelog(path, error) => {
                gha::error(format!(
                    "Unexpected I/O error while reading {}:\n{error}",
                    path.display()
                ))
                .call();
            }
            ActionError::Environment(name) => {
                gha::error(format!("Missing environment variable `{name}`")).call();
            }
            ActionError::WriteStepSummary(error) => {
                gha::error(format!(
                    "Failed to write to $GITHUB_STEP_SUMMARY:\n`{error}`"
                ))
                .call();
            }
            ActionError::Command(error) => {
                gha::error("Error executing command:\n{error}").call();
            }
        }
        std::process::exit(1);
    }
}

fn execute_action() -> Result<(), ActionError> {
    let changelog_files_input = get_multiline_input("changelog_files")
        .trim_whitespace(true)
        .call()
        .map_err(ActionError::Input)?;

    let validate_contents_input = get_boolean_input("validate_contents")
        .call()
        .map_err(ActionError::Input)?;

    let validate_touched_input = get_boolean_input("validate_touched")
        .call()
        .map_err(ActionError::Input)?;

    let validate_unreleased_input = get_boolean_input("validate_unreleased")
        .call()
        .map_err(ActionError::Input)?;

    let mut validation_reports = vec![];

    for file_glob in changelog_files_input {
        for changelog_file in glob(&file_glob)
            .map_err(ActionError::Pattern)?
            .filter_map(Result::ok)
            .filter(|path| path.is_file())
        {
            gha::debug(format!("Processing {}", changelog_file.display()));

            let mut validation_report = ValidationReport::new(changelog_file);

            let contents = fs::read_to_string(&validation_report.changelog_file).map_err(|e| {
                ActionError::ReadChangelog(validation_report.changelog_file.to_path_buf(), e)
            })?;

            match Changelog::from_str(&contents) {
                Ok(changelog) => {
                    if validate_contents_input {
                        validation_report.contents_validation = ContentsValidation::Pass;
                    }

                    if validate_unreleased_input {
                        if changelog.unreleased.changes.is_empty() {
                            validation_report.unreleased_validation = UnreleasedValidation::Fail;
                        } else {
                            validation_report.unreleased_validation = UnreleasedValidation::Pass;
                        }
                    }
                }
                Err(diagnostics) => {
                    validation_report.contents_validation = ContentsValidation::Fail(diagnostics);
                }
            };

            if validate_touched_input {
                let base_ref = gha::github_base_ref()?;

                Command::new("git")
                    .args(["fetch", "origin", &base_ref, "--depth", "1"])
                    .named_output()
                    .map_err(ActionError::Command)?;

                let diff = Command::new("git")
                    .args(["diff", &format!("remotes/origin/{base_ref}"), "--name-only"])
                    .named_output()
                    .map_err(ActionError::Command)?;

                let file_found_in_diff = diff.stdout_lossy().lines().any(|line| {
                    line.contains(validation_report.changelog_file.to_string_lossy().as_ref())
                });

                if file_found_in_diff {
                    validation_report.touched_validation = TouchedValidation::Pass;
                } else {
                    validation_report.touched_validation = TouchedValidation::Fail;
                }
            }

            validation_reports.push(validation_report);
        }
    }

    let mut summary_writer = github_step_summary()
        .and_then(|path| File::open(path).map_err(ActionError::WriteStepSummary))
        .map(BufWriter::new)?;

    for validation_report in validation_reports {
        write!(summary_writer, "{validation_report}\n\n").map_err(ActionError::WriteStepSummary)?;
    }

    Ok(())
}

#[derive(Debug)]
enum ActionError {
    Input(InputError),
    Pattern(PatternError),
    ReadChangelog(PathBuf, std::io::Error),
    Environment(String),
    WriteStepSummary(std::io::Error),
    Command(fun_run::CmdError),
}

struct ValidationReport {
    changelog_file: PathBuf,
    contents_validation: ContentsValidation,
    touched_validation: TouchedValidation,
    unreleased_validation: UnreleasedValidation,
}

impl ValidationReport {
    fn new(changelog_file: PathBuf) -> Self {
        Self {
            changelog_file,
            contents_validation: ContentsValidation::Skipped,
            touched_validation: TouchedValidation::Skipped,
            unreleased_validation: UnreleasedValidation::Skipped,
        }
    }
}

const SKIP_EMOTICON: &str = ":white_circle:";
const SKIP_TEXT: &str = "(skip)";
const PASS_EMOTICON: &str = ":large_blue_circle";
const PASS_TEXT: &str = "(pass)";
const FAIL_EMOTICON: &str = ":red_circle:";
const FAIL_TEXT: &str = "(fail)";
const TOUCHED_VALIDATION: &str = "Check: Has the Changelog been touched";
const UNRELEASED_VALIDATION: &str = "Check: Does the Changelog contains unreleased changes";
const CONTENTS_VALIDATION: &str = "Check: Is the Changelog format valid";

impl Display for ValidationReport {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let pass =
            |f: &mut Formatter, message| writeln!(f, "- {PASS_EMOTICON} {message} {PASS_TEXT}\n");
        let fail =
            |f: &mut Formatter, message| writeln!(f, "- {FAIL_EMOTICON} {message} {FAIL_TEXT}\n");
        let skip =
            |f: &mut Formatter, message| writeln!(f, "- {SKIP_EMOTICON} {message} {SKIP_TEXT}\n");

        write!(f, "### `{}`\n\n", self.changelog_file.display())?;

        match self.touched_validation {
            TouchedValidation::Skipped => skip(f, TOUCHED_VALIDATION),
            TouchedValidation::Pass => pass(f, TOUCHED_VALIDATION),
            TouchedValidation::Fail => fail(f, TOUCHED_VALIDATION),
        }?;

        match self.unreleased_validation {
            UnreleasedValidation::Skipped => skip(f, UNRELEASED_VALIDATION),
            UnreleasedValidation::Pass => pass(f, UNRELEASED_VALIDATION),
            UnreleasedValidation::Fail => fail(f, UNRELEASED_VALIDATION),
        }?;

        match self.contents_validation {
            ContentsValidation::Skipped => skip(f, CONTENTS_VALIDATION),
            ContentsValidation::Pass => pass(f, CONTENTS_VALIDATION),
            ContentsValidation::Fail(_) => fail(f, CONTENTS_VALIDATION),
        }?;

        if let ContentsValidation::Fail(diagnostics) = &self.contents_validation {
            writeln!(f)?;
            writeln!(f, "| Line | Column | Error |\n")?;
            writeln!(f, "|-----:|-------:|-------|\n")?;
            for diagnostic in diagnostics {
                writeln!(
                    f,
                    "| {line} | {column} | <pre>{message}</pre> |\n",
                    message = diagnostic.message,
                    line = diagnostic.position.start.line,
                    column = diagnostic.position.start.column
                )?;
            }
        }

        Ok(())
    }
}

enum ContentsValidation {
    Skipped,
    Pass,
    Fail(Vec<Diagnostic>),
}

enum TouchedValidation {
    Skipped,
    Pass,
    Fail,
}

enum UnreleasedValidation {
    Skipped,
    Pass,
    Fail,
}

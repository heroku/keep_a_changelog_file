mod gha;

use crate::gha::{get_multiline_input, InputError};
use glob::{glob, PatternError};

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
            println!("{}", entry.to_string_lossy());
        }
    }
    Ok(())
}

enum ActionError {
    Input(InputError),
    Pattern(PatternError),
}

use crate::ActionError;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::path::PathBuf;

#[bon::builder]
pub(crate) fn get_input(
    #[builder(start_fn)] //
    name: &str,
    #[builder(default = false)] //
    required: bool,
    #[builder(default = false)] trim_whitespace: bool,
) -> Result<String, InputError> {
    let name = format!("INPUT_{}", name.replace(" ", "_").to_ascii_uppercase());
    match std::env::var(&name) {
        Ok(val) => {
            if trim_whitespace {
                Ok(val.trim().to_string())
            } else {
                Ok(val)
            }
        }
        Err(_) => {
            if required {
                Err(InputError::MissingRequiredValue(name.to_string()))
            } else {
                Ok(String::default())
            }
        }
    }
}

#[bon::builder]
pub(crate) fn get_multiline_input(
    #[builder(start_fn)] //
    name: &str,
    #[builder(default = false)] //
    required: bool,
    #[builder(default = false)] trim_whitespace: bool,
) -> Result<Vec<String>, InputError> {
    let value = get_input(name)
        .required(required)
        .trim_whitespace(trim_whitespace)
        .call()?;
    Ok(value
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            if trim_whitespace {
                line.trim().to_string()
            } else {
                line.to_string()
            }
        })
        .collect())
}

const TRUE_VALUES: [&str; 3] = ["true", "True", "TRUE"];
const FALSE_VALUES: [&str; 3] = ["false", "False", "FALSE"];

#[bon::builder]
pub(crate) fn get_boolean_input(
    #[builder(start_fn)] //
    name: &str,
    #[builder(default = false)] //
    required: bool,
    #[builder(default = false)] trim_whitespace: bool,
) -> Result<bool, InputError> {
    let value = get_input(name)
        .required(required)
        .trim_whitespace(trim_whitespace)
        .call()?;

    if TRUE_VALUES.contains(&value.as_str()) {
        return Ok(true);
    }

    if FALSE_VALUES.contains(&value.as_str()) {
        return Ok(false);
    }

    Err(InputError::InvalidBooleanType(name.to_string()))
}

#[derive(Debug)]
pub(crate) enum InputError {
    MissingRequiredValue(String),
    InvalidBooleanType(String),
}

pub(crate) fn github_step_summary() -> Result<PathBuf, ActionError> {
    std::env::var("GITHUB_STEP_SUMMARY")
        .map(PathBuf::from)
        .map_err(|_| ActionError::Environment("GITHUB_STEP_SUMMARY".to_string()))
}

pub(crate) fn github_base_ref() -> Result<String, ActionError> {
    std::env::var("GITHUB_BASE_REF")
        .map(String::from)
        .map_err(|_| ActionError::Environment("GITHUB_BASE_REF".to_string()))
}

pub(crate) fn debug(message: impl Into<String>) {
    issue_command(Command {
        command: CommandType::Debug,
        message: message.into(),
        properties: CommandProperties {
            inner: HashMap::new(),
        },
    })
}

#[bon::builder]
pub(crate) fn error(
    #[builder(start_fn, into)] //
    message: String,
    #[builder(into)] //
    title: Option<String>,
    file: Option<PathBuf>,
    start_line: Option<usize>,
    end_line: Option<usize>,
    start_column: Option<usize>,
    end_column: Option<usize>,
) {
    issue_command(Command {
        command: CommandType::Error,
        properties: AnnotationProperties::builder()
            .maybe_title(title)
            .maybe_file(file)
            .maybe_start_line(start_line)
            .maybe_end_line(end_line)
            .maybe_start_column(start_column)
            .maybe_end_column(end_column)
            .build()
            .into(),
        message,
    });
}

fn issue_command(command: Command) {
    println!("{command}");
}

struct Command {
    command: CommandType,
    message: String,
    properties: CommandProperties,
}

const CMD_STRING: &str = "::";

impl Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{CMD_STRING}{}", self.command)?;
        if !self.properties.is_empty() {
            write!(f, " ")?;
        }
        let mut property_strings = vec![];
        for (key, value) in self.properties.deref() {
            property_strings.push(format!("{}={}", key, escape_property(value)));
        }
        write!(f, "{}", property_strings.join(","))?;
        write!(f, "{CMD_STRING}{}", escape_data(&self.message))
    }
}

enum CommandType {
    Error,
    Debug,
}

impl Display for CommandType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandType::Error => write!(f, "error"),
            CommandType::Debug => write!(f, "debug"),
        }
    }
}

struct CommandProperties {
    inner: HashMap<String, String>,
}

impl Deref for CommandProperties {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(bon::Builder)]
struct AnnotationProperties {
    title: Option<String>,
    file: Option<PathBuf>,
    start_line: Option<usize>,
    end_line: Option<usize>,
    start_column: Option<usize>,
    end_column: Option<usize>,
}

impl From<AnnotationProperties> for CommandProperties {
    fn from(props: AnnotationProperties) -> Self {
        let mut inner = HashMap::new();
        if let Some(title) = props.title {
            inner.insert("title".to_string(), escape_property(&title));
        }
        if let Some(file) = props.file {
            inner.insert("file".to_string(), escape_property(&file.to_string_lossy()));
        }
        if let Some(start_line) = props.start_line {
            inner.insert(
                "startLine".to_string(),
                escape_property(&start_line.to_string()),
            );
        }
        if let Some(end_line) = props.end_line {
            inner.insert(
                "endLine".to_string(),
                escape_property(&end_line.to_string()),
            );
        }
        if let Some(start_column) = props.start_column {
            inner.insert(
                "startColumn".to_string(),
                escape_property(&start_column.to_string()),
            );
        }
        if let Some(end_column) = props.end_column {
            inner.insert(
                "endColumn".to_string(),
                escape_property(&end_column.to_string()),
            );
        }
        CommandProperties { inner }
    }
}

fn escape_data(data: &str) -> String {
    data.replace("%", "%25")
        .replace("\r", "%0D")
        .replace("\n", "%0A")
}

fn escape_property(data: &str) -> String {
    data.replace("%", "%25")
        .replace("\r", "%0D")
        .replace("\n", "%0A")
        .replace(":", "%3A")
        .replace(",", "%2C")
}

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

pub(crate) enum InputError {
    MissingRequiredValue(String),
}

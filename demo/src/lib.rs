use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn print_syntax_tree(text: &str) -> String {
    keep_a_changelog_file::__printable_syntax_tree(text)
}

#[wasm_bindgen]
pub fn get_errors(text: &str) -> JsValue {
    serde_wasm_bindgen::to_value(&match keep_a_changelog_file::Changelog::from_str(text) {
        Ok(_) => vec![],
        Err(errors) => errors
            .iter()
            .map(|diagnostic| JsDiagnostic {
                message: diagnostic.message.clone(),
                start: Point {
                    line: diagnostic.position.start.line,
                    column: diagnostic.position.start.column,
                    offset: diagnostic.position.start.offset,
                },
                end: Point {
                    line: diagnostic.position.end.line,
                    column: diagnostic.position.end.column,
                    offset: diagnostic.position.end.offset,
                },
            })
            .collect::<Vec<_>>(),
    })
    .unwrap()
}

#[derive(Serialize, Deserialize)]
pub struct JsDiagnostic {
    pub message: String,
    pub start: Point,
    pub end: Point,
}

#[derive(Serialize, Deserialize)]
pub struct Point {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

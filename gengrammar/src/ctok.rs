//! Tokenizer for the C subset tree-sitter writes into `parser.c` (port of `ctok.go`).

use std::collections::HashMap;
use std::fmt;

/// One token. `Char` carries the value and the source text, `Str` the decoded bytes.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Eof,
    Ident(String),
    Num(i64),
    Char(i64, String),
    Str(Vec<u8>),
    Punct(&'static str),
}

/// A token and its byte offset in the source.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub pos: usize,
}

/// A generator error. The message is `parser.c:<line>: <msg>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenError(pub String);

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GenError {}

/// The tokenizer; `#define NAME <integer>` lines land in `defines`.
pub struct Tokenizer {
    pub defines: HashMap<String, i64>,
}

impl Tokenizer {
    pub fn new(_src: &str) -> Self {
        todo!()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Token, GenError> {
        todo!()
    }
}

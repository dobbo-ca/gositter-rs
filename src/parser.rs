//! parser.c.

use crate::{Language, Logger, Tree};

/// TSParser. One per thread at a time; it can move between threads.
pub struct Parser {
    _private: (),
}

/// `set_language` was given a grammar of an ABI this runtime does not accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageError {
    pub abi_version: u32,
}

impl std::fmt::Display for LanguageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "gositter: incompatible language ABI version {}",
            self.abi_version
        )
    }
}

impl std::error::Error for LanguageError {}

impl Default for Parser {
    fn default() -> Self {
        Parser::new()
    }
}

impl Parser {
    /// ts_parser_new
    pub fn new() -> Parser {
        todo!("gositter-rs-2f0.11")
    }

    /// ts_parser_set_language. Errors on an incompatible ABI version.
    pub fn set_language(&mut self, _language: &'static Language) -> Result<(), LanguageError> {
        todo!("gositter-rs-2f0.11")
    }

    /// ts_parser_set_logger. `None` turns logging off.
    pub fn set_logger(&mut self, _logger: Option<Logger>) {
        todo!("gositter-rs-2f0.11")
    }

    /// ts_parser_reset
    pub fn reset(&mut self) {
        todo!("gositter-rs-2f0.11")
    }

    /// ts_parser_parse with no old tree and UTF-8 input. `None` if no language is set.
    pub fn parse(&mut self, _src: &[u8]) -> Option<&Tree> {
        todo!("gositter-rs-2f0.11")
    }

    /// `parse`, but `None` once advance ran more than `max_stall` times in a row without
    /// passing the furthest byte reached; the parser is then reset. 0 is no limit. The
    /// count depends only on the language and `src`.
    pub fn parse_budget(&mut self, _src: &[u8], _max_stall: u32) -> Option<&Tree> {
        todo!("gositter-rs-2f0.11")
    }
}

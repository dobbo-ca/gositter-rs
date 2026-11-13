//! Lexer emitter (stub).

use crate::ctok::GenError;
use crate::parse::Grammar;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexForm {
    Code,
    Table,
}

#[allow(dead_code)] // tests-first stub
pub fn emit_lex(g: &Grammar, form: LexForm) -> Result<(String, Vec<u8>), GenError> {
    let _ = (g, form);
    todo!()
}

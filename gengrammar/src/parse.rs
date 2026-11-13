//! Parser of the C subset of `parser.c` into a [`Grammar`] (port of `parse.go`).

use std::collections::{BTreeMap, HashMap};

use crate::cond::Cond;
use crate::ctok::{GenError, Token};

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub names: Vec<String>,
    pub vals: Vec<i64>,
}

/// One static array, flattened row-major. Struct elements take one value per field.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub name: String,
    pub ctype: String,
    /// As declared; 0 is an open dimension.
    pub dims: Vec<usize>,
    pub fields: Option<&'static [&'static str]>,
    pub vals: Vec<i64>,
    /// ctype `char*` only.
    pub strs: Vec<String>,
}

#[allow(clippy::len_without_is_empty)]
impl Table {
    pub fn width(&self) -> usize {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    If { cond: Cond, skip: bool, target: u16 },
    Map(Vec<(u16, u16)>),
    Accept(String),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexCase {
    pub state: u16,
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexFunc {
    pub cname: String,
    pub cases: Vec<LexCase>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Grammar {
    /// The `<name>` of `tree_sitter_<name>`.
    pub name: String,
    pub defines: HashMap<String, i64>,
    /// Enum members and builtin symbols.
    pub consts: HashMap<String, i64>,
    pub enums: Vec<EnumDecl>,
    pub tables: HashMap<String, Table>,
    /// Table names in source order.
    pub order: Vec<String>,
    /// `ts_parse_actions`, 5 values per slot.
    pub actions: Vec<i64>,
    pub lex_fns: Vec<LexFunc>,
    /// The `TSLanguage` initialiser, nested keys joined with `.`.
    pub lang: BTreeMap<String, Vec<Token>>,
}

pub fn parse_grammar(_src: &str) -> Result<Grammar, GenError> {
    todo!()
}

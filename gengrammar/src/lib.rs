//! Port of gositter v0.2.0 `cmd/gengrammar`: reads the C subset of a tree-sitter `parser.c`,
//! checks its sha256 against `grammars.lock`, and writes the tables of one grammar. No C
//! compiler is involved. The binary in `main.rs` is the command line over this library.

pub mod blob;
pub mod cond;
pub mod ctok;
pub mod lexgen;
pub mod lock;
pub mod oracle;
pub mod parse;
pub mod rust;
pub mod sha256;

//! Pure-Rust port of [gositter](https://github.com/dobbo-ca/gositter) v0.2.0, itself a
//! pure-Go port of the tree-sitter v0.25.0 runtime. Plan:
//! `docs/superpowers/plans/2026-11-13-runtime.md`.

mod api;
mod node;
mod parser;
mod tree;
mod tree_cursor;

pub use api::*;
pub use node::Node;
pub use parser::{LanguageError, Parser};
pub use tree::Tree;
pub use tree_cursor::TreeCursor;

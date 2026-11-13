//! Pure-Rust port of [gositter](https://github.com/dobbo-ca/gositter) v0.2.0, itself a
//! pure-Go port of the tree-sitter v0.25.0 runtime. Plan:
//! `docs/superpowers/plans/2026-11-13-runtime.md`.

mod api;
mod export;
// The parser (plan Task 11) is the first user of language.rs's internals.
#[allow(dead_code)]
mod language;
// The parser (plan Task 11) is the first user of these two; until then only their tests are.
#[allow(dead_code)]
mod length;
#[allow(dead_code)]
mod lexer;
// Stubs until Task 10b.
#[allow(dead_code)]
mod node;
mod parser;
// The parser (plan Task 11) is the first user of subtree.rs's internals.
#[allow(dead_code)]
mod subtree;
// The parser (plan Task 11) is the first user of stack.rs's internals.
#[allow(dead_code)]
mod stack;
#[cfg(test)]
mod test_language;
// Stubs until Task 10b.
#[allow(dead_code)]
mod tree;
// Stubs until Task 10b.
#[allow(dead_code)]
mod tree_cursor;

pub use api::*;
pub use node::Node;
pub use parser::{LanguageError, Parser};
pub use tree::Tree;
pub use tree_cursor::TreeCursor;

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
// Tree::empty and Tree::from_pool wait for the parser (plan Task 11).
#[allow(dead_code)]
mod tree;
mod tree_cursor;

pub use api::*;
#[doc(hidden)]
pub use export::SummaryCheck;
pub use node::Node;
pub use parser::{LanguageError, Parser};
#[doc(hidden)]
pub use stack::{SHRINK_LINKS, SHRINK_NODES};
#[doc(hidden)]
pub use subtree::{KEPT_HEAP_CHUNKS, SHRINK_CHILDREN};
pub use tree::Tree;
pub use tree_cursor::TreeCursor;

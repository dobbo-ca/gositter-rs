//! Pure-Rust port of [gositter](https://github.com/dobbo-ca/gositter) v0.2.0, itself a
//! pure-Go port of the tree-sitter v0.25.0 runtime.

/// The newest grammar ABI version the runtime accepts (TREE_SITTER_LANGUAGE_VERSION).
pub const TREE_SITTER_LANGUAGE_VERSION: u32 = 15;

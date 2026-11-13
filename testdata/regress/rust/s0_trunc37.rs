//! This crate provides Lua language support for the [tree-sitter] parsing library.
//!
//! Typically, you will use the [`LANGUAGE`] constant to add this language to a
//! tree-sitter [`Parser`], and then use the parser to parse some code:
//!
//! ```
//! let code = r#"
//! return 42
//! "#;
//! let mut parser = tree_sitter::Parser::new();
//! let language = tree_sitter_lua::LANGUAGE;
//! parser
//!     .set_language(&language.into())
//!     .expect("Error loading Lua parser");
//! let tree = parser.parse(code, None).unwrap();
//! assert!(!tree.root_node().has_error());
//! ```
//!
//! [`Parser`]: https://docs.rs/tree-sitter/0.25.3/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

use
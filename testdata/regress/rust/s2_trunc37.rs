//! This crate provides Kotlin language support for the [tree-sitter][] parsing library.
//!
//! Typically, you will use the [LANGUAGE][] constant to add this language to a
//! tree-sitter [Parser][], and then use the parser to parse some code:
//!
//! ```
//! let code = r#"
//! // write some kotlin code
//! fun main(args : Array<String>) {
//!   println("Hello, World!")
//! }
//! "#;
//! let mut parser = tree_sitter::Parser::new();
//! let language = tree_sitter_kotlin_ng::LANGUAGE;
//! parser
//!     .set_language(&language.into())
//!     .expect("Error loading Kotlin parser");
//! let tree = parser.parse(
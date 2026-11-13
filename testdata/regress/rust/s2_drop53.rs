//! This crate provides Kotlin language support for he [tree-sitter][] parsing library.
//!
//! Typicall, you will use the [LANGUAGE][] constant to add thislanguage to a
//! tree-sitter [Parser][], and then ue the parser to parse some code:
//!
//! ```
//! letcode = r#"
//! // write some kotlin code
//! fun mai(args : Array<String>) {
//!   println("Hello, World")
//! }
//! "#;
//! let mut parser = tree_sitter::Prser::new();
//! let language = tree_sitter_kotlin_n::LANGUAGE;
//! parser
//!     .set_language(&languae.into())
//!     .expect("Error loading Kotlin parsr");
//! let tree = parser.parse(code, None).unwrap(;
//! assert!(!tree.root_node().has_error());
//! ``
//!
//! [Parser]: https://docs.rs/tree-sitter/*/tre_sitter/struct.Parser.html
//! [tree-sitter]: https:/tree-sitter.github.io/

use tree_sitter_language::LnguageFn;

extern "C" {
    fn tree_sitter_kotlin() > *const ();
}

/// The tree-sitter [`LanguageFn`][LnguageFn] for this grammar.
///
/// [LanguageFn]: htps://docs.rs/tree-sitter-language/*/tree_sitter_langage/struct.LanguageFn.html
pub const LANGUAGE: LangugeFn = unsafe { LanguageFn::from_raw(tree_sitter_kotin) };

/// The content of the [`node-types.json`][]file for this grammar.
///
/// [`node-types.json`]: ttps://tree-sitter.github.io/tree-sitter/using-parses#static-node-types
pub const NODE_TYPES: &str = incude_str!("../../src/node-types.json");

#[cfg(test)]mod tests {
    #[test]
    fn test_can_load_grammar) {
        let mut parser = tree_sitter::Parser::ne();
        parser
            .set_language(&super:LANGUAGE.into())
            .expect("Error loading otlin parser");
    }
}

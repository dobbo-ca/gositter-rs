//! Port of tree-sitter-julia v0.25.0 `src/scanner.c` (gositter `grammars/julia/scanner.go`). The scanner has no state: C create returns NULL, serialize writes nothing and
//! deserialize ignores its input. `scan` waits for the runtime Lexer (Task 12).

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner;

// enum TokenType
#[allow(dead_code)] // used by scan (Task 12)
const TOK_BLOCK_COMMENT_REST: usize = 0;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_IMMEDIATE_PAREN: usize = 1;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_IMMEDIATE_BRACKET: usize = 2;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_IMMEDIATE_BRACE: usize = 3;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_IMMEDIATE_STRING_START: usize = 4;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_IMMEDIATE_COMMAND_START: usize = 5;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CONTENT_CMD1: usize = 6;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CONTENT_CMD1_RAW: usize = 7;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CONTENT_CMD3: usize = 8;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CONTENT_CMD3_RAW: usize = 9;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CONTENT_STR1: usize = 10;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CONTENT_STR1_RAW: usize = 11;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CONTENT_STR3: usize = 12;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_CONTENT_STR3_RAW: usize = 13;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_END_CMD: usize = 14;
#[allow(dead_code)] // used by scan (Task 12)
const TOK_END_STR: usize = 15;

impl Scanner {
    // tree_sitter_julia_external_scanner_create
    pub fn new() -> Self {
        Scanner
    }

    // tree_sitter_julia_external_scanner_serialize
    pub fn serialize(&self, _buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        0
    }

    // tree_sitter_julia_external_scanner_deserialize
    pub fn deserialize(&mut self, _buf: &[u8]) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    // C serialize returns 0 and deserialize ignores its input: every scanner state must serialize
    // to the same empty byte string.
    // scanner_test.go TestSerializeRoundTrip
    #[test]
    fn serialize_round_trip() {
        let mut s = Scanner::new();
        let mut buf = [0xAAu8; crate::SERIALIZATION_BUFFER_SIZE];
        assert_eq!(s.serialize(&mut buf), 0, "Serialize != 0");
        for (i, &b) in buf.iter().enumerate() {
            assert_eq!(b, 0xAA, "Serialize wrote buf[{i}]");
        }
        s.deserialize(&[]);
        s.deserialize(&[1, 2, 3]);
        assert_eq!(s.serialize(&mut buf), 0, "Serialize after Deserialize != 0");
    }
}

#[cfg(test)]
mod parser_tests {
    /// The grammar's generated language() (grammars plan Task 10, bead gositter-rs-vjz.10). The tests below are ignored until then.
    fn language() -> &'static gositter::Language {
        unimplemented!("gositter-rs-vjz.10: generated language()")
    }

    // scanner_test.go TestLanguageWiresScanner
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn language_wires_scanner() {
        assert!(
            language().tables.external_scanner.create.is_some(),
            "ExternalScanner.New is nil: rerun grammars/gen.sh julia"
        );
    }

    // scanner_test.go TestParseScannerTokens
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn parse_scanner_tokens() {
        let mut p = gositter::Parser::new();
        p.set_language(language()).unwrap();
        let src = "#= a #= b =# c =#\nf(x) = \"a$(x)b\" * raw\"\\\"q\" * `ls $x` * \"\"\"t \"u\" \"\"\"\n";
        let tree = p.parse(src.as_bytes()).unwrap();
        let got = tree.root_node().to_sexp();
        assert!(
            !got.is_empty() && !tree.root_node().has_error(),
            "tree = {got}"
        );
    }
}

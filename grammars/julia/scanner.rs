//! Port of tree-sitter-julia v0.25.0 `src/scanner.c` (gositter `grammars/julia/scanner.go`). The scanner has no state: C create returns NULL, serialize writes nothing and
//! deserialize ignores its input. `scan` is ported below.

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner;

// enum TokenType
const TOK_BLOCK_COMMENT_REST: usize = 0;
const TOK_IMMEDIATE_PAREN: usize = 1;
const TOK_IMMEDIATE_BRACKET: usize = 2;
const TOK_IMMEDIATE_BRACE: usize = 3;
const TOK_IMMEDIATE_STRING_START: usize = 4;
const TOK_IMMEDIATE_COMMAND_START: usize = 5;
const TOK_CONTENT_CMD1: usize = 6;
const TOK_CONTENT_CMD1_RAW: usize = 7;
const TOK_CONTENT_CMD3: usize = 8;
const TOK_CONTENT_CMD3_RAW: usize = 9;
const TOK_CONTENT_STR1: usize = 10;
const TOK_CONTENT_STR1_RAW: usize = 11;
const TOK_CONTENT_STR3: usize = 12;
const TOK_CONTENT_STR3_RAW: usize = 13;
const TOK_END_CMD: usize = 14;
const TOK_END_STR: usize = 15;

// scan_content
fn scan_content(
    lexer: &mut gositter::Lexer<'_>,
    content_symbol: gositter::Symbol,
    end_char: i32,
    n_delim: u32,
    interp: bool,
) -> bool {
    let mut end_symbol = TOK_END_CMD as gositter::Symbol;
    if end_char == '"' as i32 {
        end_symbol = TOK_END_STR as gositter::Symbol;
    }
    let mut has_content = false;
    loop {
        let mut next = lexer.lookahead;
        if next == 0 {
            break;
        }
        lexer.mark_end();
        if interp && (next == '$' as i32 || next == '\\' as i32) {
            lexer.result_symbol = content_symbol;
            return has_content;
        } else if next == '\\' as i32 {
            // Raw strings: only \<delimiter> and \\ end the content.
            lexer.advance(false);
            next = lexer.lookahead;
            if next == end_char || next == '\\' as i32 {
                lexer.result_symbol = content_symbol;
                return has_content;
            }
        } else {
            let mut is_end_delimiter = true;
            for _ in 1..=n_delim {
                if lexer.lookahead == end_char {
                    lexer.advance(false);
                } else {
                    is_end_delimiter = false;
                    break;
                }
            }
            if is_end_delimiter {
                if has_content {
                    lexer.result_symbol = content_symbol;
                    return true;
                }
                lexer.mark_end();
                lexer.result_symbol = end_symbol;
                return true;
            }
        }
        lexer.advance(false);
        has_content = true;
    }
    false
}

// scan_block_comment
fn scan_block_comment(lexer: &mut gositter::Lexer<'_>) -> bool {
    // The first `#=` is scanned by the generated lexer.
    let mut after_eq = false;
    let mut nesting_depth = 1u32;
    loop {
        match char::from_u32(lexer.lookahead as u32) {
            Some('=') => {
                lexer.advance(false);
                after_eq = true;
            }
            Some('#') => {
                lexer.advance(false);
                if after_eq {
                    after_eq = false;
                    nesting_depth -= 1;
                    if nesting_depth == 0 {
                        lexer.result_symbol = TOK_BLOCK_COMMENT_REST as gositter::Symbol;
                        return true;
                    }
                } else {
                    after_eq = false;
                    if lexer.lookahead == '=' as i32 {
                        lexer.advance(false);
                        nesting_depth += 1;
                    }
                }
            }
            Some('\0') => return false,
            _ => {
                lexer.advance(false);
                after_eq = false;
            }
        }
    }
}

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

impl gositter::ExternalScanner for Scanner {
    // tree_sitter_julia_external_scanner_scan
    fn scan(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        if valid_symbols[TOK_IMMEDIATE_PAREN] && lexer.lookahead == '(' as i32 {
            lexer.result_symbol = TOK_IMMEDIATE_PAREN as gositter::Symbol;
            return true;
        } else if valid_symbols[TOK_IMMEDIATE_BRACKET] && lexer.lookahead == '[' as i32 {
            lexer.result_symbol = TOK_IMMEDIATE_BRACKET as gositter::Symbol;
            return true;
        } else if valid_symbols[TOK_IMMEDIATE_BRACE] && lexer.lookahead == '{' as i32 {
            lexer.result_symbol = TOK_IMMEDIATE_BRACE as gositter::Symbol;
            return true;
        } else if valid_symbols[TOK_IMMEDIATE_STRING_START] && lexer.lookahead == '"' as i32 {
            lexer.result_symbol = TOK_IMMEDIATE_STRING_START as gositter::Symbol;
            return true;
        } else if valid_symbols[TOK_IMMEDIATE_COMMAND_START] && lexer.lookahead == '`' as i32 {
            lexer.result_symbol = TOK_IMMEDIATE_COMMAND_START as gositter::Symbol;
            return true;
        }

        if valid_symbols[TOK_BLOCK_COMMENT_REST] && scan_block_comment(lexer) {
            return true;
        }

        // Each failed attempt leaves the lexer advanced; the next one starts
        // from there, as in C.
        if valid_symbols[TOK_CONTENT_STR1]
            && scan_content(
                lexer,
                TOK_CONTENT_STR1 as gositter::Symbol,
                '"' as i32,
                1,
                true,
            )
        {
            return true;
        }
        if valid_symbols[TOK_CONTENT_STR3]
            && scan_content(
                lexer,
                TOK_CONTENT_STR3 as gositter::Symbol,
                '"' as i32,
                3,
                true,
            )
        {
            return true;
        }
        if valid_symbols[TOK_CONTENT_CMD1]
            && scan_content(
                lexer,
                TOK_CONTENT_CMD1 as gositter::Symbol,
                '`' as i32,
                1,
                true,
            )
        {
            return true;
        }
        if valid_symbols[TOK_CONTENT_CMD3]
            && scan_content(
                lexer,
                TOK_CONTENT_CMD3 as gositter::Symbol,
                '`' as i32,
                3,
                true,
            )
        {
            return true;
        }
        if valid_symbols[TOK_CONTENT_STR1_RAW]
            && scan_content(
                lexer,
                TOK_CONTENT_STR1_RAW as gositter::Symbol,
                '"' as i32,
                1,
                false,
            )
        {
            return true;
        }
        if valid_symbols[TOK_CONTENT_STR3_RAW]
            && scan_content(
                lexer,
                TOK_CONTENT_STR3_RAW as gositter::Symbol,
                '"' as i32,
                3,
                false,
            )
        {
            return true;
        }
        if valid_symbols[TOK_CONTENT_CMD1_RAW]
            && scan_content(
                lexer,
                TOK_CONTENT_CMD1_RAW as gositter::Symbol,
                '`' as i32,
                1,
                false,
            )
        {
            return true;
        }
        if valid_symbols[TOK_CONTENT_CMD3_RAW]
            && scan_content(
                lexer,
                TOK_CONTENT_CMD3_RAW as gositter::Symbol,
                '`' as i32,
                3,
                false,
            )
        {
            return true;
        }

        false
    }

    fn serialize(&self, buf: &mut [u8; gositter::SERIALIZATION_BUFFER_SIZE]) -> usize {
        Scanner::serialize(self, buf)
    }

    fn deserialize(&mut self, buf: &[u8]) {
        Scanner::deserialize(self, buf)
    }
}

#[allow(dead_code)] // used by the generated language() (Task 10)
pub(crate) fn new() -> Box<dyn gositter::ExternalScanner> {
    Box::new(Scanner::new())
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

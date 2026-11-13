//! Port of gositter `grammars/lua/scanner.go` (tree-sitter-lua v0.5.0 `src/scanner.c`): the scanner
//! state, `serialize` and `deserialize`. `scan` waits for the runtime `Lexer` (Task 12).

/// C never assigns `ending_char` a non-zero value; it only comes back from `deserialize`, so it is
/// 0 on every reachable path. Kept for the bytes.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner {
    /// `char`, signed on the oracle's targets.
    ending_char: i8,
    level_count: u8,
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        buf[0] = self.ending_char as u8;
        buf[1] = self.level_count;
        2
    }

    /// Length 0 leaves the state as it is, as C does.
    pub fn deserialize(&mut self, buf: &[u8]) {
        if buf.is_empty() {
            return;
        }
        self.ending_char = buf[0] as i8;
        if buf.len() == 1 {
            return;
        }
        self.level_count = buf[1];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TestSerializeRoundTrip
    #[test]
    fn serialize_round_trip() {
        let mut buf = [0u8; crate::SERIALIZATION_BUFFER_SIZE];
        let mut s = Scanner::new();
        let n = s.serialize(&mut buf);
        assert_eq!(&buf[..n], [0, 0], "fresh scanner");

        let cases: [[u8; 2]; 6] = [[0, 0], [0, 3], [0, 255], [b'"', 7], [0x80, 1], [0xFF, 0xFF]];
        for want in cases {
            // Dirty target.
            let mut got = Scanner {
                ending_char: 5,
                level_count: 9,
            };
            got.deserialize(&want);
            let n = got.serialize(&mut buf);
            assert_eq!(&buf[..n], want, "round trip of {want:?}");
        }
        // char is signed: 0x80 must not compare equal to code point U+0080.
        s.deserialize(&[0x80, 0]);
        assert_eq!(s.ending_char, -128);
    }

    // TestDeserializeShort
    #[test]
    fn deserialize_short() {
        // C returns early: length 0 touches nothing, length 1 only ending_char.
        let mut s = Scanner {
            ending_char: 5,
            level_count: 9,
        };
        s.deserialize(&[]);
        assert!(
            s.ending_char == 5 && s.level_count == 9,
            "length 0 changed the state: {s:?}"
        );
        s.deserialize(&[7]);
        assert!(s.ending_char == 7 && s.level_count == 9, "length 1: {s:?}");
    }
}

#[cfg(test)]
mod parser_tests {
    /// The grammar's generated language() (grammars plan Task 10, bead gositter-rs-vjz.10). The tests below are ignored until then.
    fn language() -> &'static gositter::Language {
        unimplemented!("gositter-rs-vjz.10: generated language()")
    }

    // Trees for long brackets, incl. the uint8 wrap at 256 '='. TestLongBrackets (scanner_test.go)
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn long_brackets() {
        let eq = |n: usize| "=".repeat(n);
        for (src, want) in [
            (
                "x = [==[a]]b]=]c]==]".to_string(),
                "(chunk (assignment_statement (variable_list name: (identifier)) (expression_list value: (string content: (string_content)))))",
            ),
            (
                "--[[ c ]] x()".to_string(),
                "(chunk (comment content: (comment_content)) (function_call name: (identifier) arguments: (arguments)))",
            ),
            // level 256 wraps to 0, so a plain ]] closes it
            (
                format!("x = [{}[a]]", eq(256)),
                "(chunk (assignment_statement (variable_list name: (identifier)) (expression_list value: (string content: (string_content)))))",
            ),
        ] {
            let mut p = gositter::Parser::new();
            p.set_language(language()).unwrap();
            let tree = p.parse(src.as_bytes()).unwrap();
            let got = tree.root_node().to_sexp();
            assert_eq!(got, want, "{src:?}");
        }
    }
}

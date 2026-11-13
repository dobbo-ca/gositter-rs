//! Port of gositter `grammars/csharp/scanner.go` (tree-sitter-c-sharp v0.23.5 `src/scanner.c`):
//! state, serialize and deserialize. `scan` waits for the runtime Lexer (Task 12).

use crate::SERIALIZATION_BUFFER_SIZE;

// enum TokenType. Only `scan` reads these.
#[allow(dead_code)]
mod tok {
    pub const OPT_SEMI: u16 = 0;
    pub const INTERPOLATION_REGULAR_START: u16 = 1;
    pub const INTERPOLATION_VERBATIM_START: u16 = 2;
    pub const INTERPOLATION_RAW_START: u16 = 3;
    pub const INTERPOLATION_START_QUOTE: u16 = 4;
    pub const INTERPOLATION_END_QUOTE: u16 = 5;
    pub const INTERPOLATION_OPEN_BRACE: u16 = 6;
    pub const INTERPOLATION_CLOSE_BRACE: u16 = 7;
    pub const INTERPOLATION_STRING_CONTENT: u16 = 8;
    pub const RAW_STRING_START: u16 = 9;
    pub const RAW_STRING_END: u16 = 10;
    pub const RAW_STRING_CONTENT: u16 = 11;
}

// enum StringType
const STRING_REGULAR: u8 = 1 << 0;
const STRING_VERBATIM: u8 = 1 << 1;
const STRING_RAW: u8 = 1 << 2;

/// Interpolation. `string_type` is a 4-byte enum in C, but only values 0..7 are ever stored and
/// serialize keeps one byte, so a byte holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct Interpolation {
    dollar_count: u8,
    open_brace_count: u8,
    quote_count: u8,
    string_type: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner {
    quote_count: u8,
    interpolation_stack: Vec<Interpolation>,
}

/// `iswspace` in the "C" locale: the six ASCII spaces, nothing above 127, not -1.
fn iswspace(c: i32) -> bool {
    let _ = c;
    todo!()
}

impl Scanner {
    /// `newScanner`
    pub fn new() -> Self {
        todo!()
    }

    /// `Serialize`
    pub fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
        let _ = (self, buf);
        todo!()
    }

    /// `Deserialize`
    pub fn deserialize(&mut self, buf: &[u8]) {
        let _ = (self, buf);
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(s: &Scanner) -> (Scanner, Vec<u8>) {
        let mut buf = [0u8; SERIALIZATION_BUFFER_SIZE];
        let n = s.serialize(&mut buf);
        // Dirty target: deserialize must replace all of it.
        let mut got = Scanner {
            quote_count: 9,
            interpolation_stack: vec![
                Interpolation {
                    dollar_count: 1,
                    open_brace_count: 2,
                    quote_count: 3,
                    string_type: 4,
                },
                Interpolation {
                    dollar_count: 5,
                    open_brace_count: 6,
                    quote_count: 7,
                    string_type: 1,
                },
            ],
        };
        got.deserialize(&buf[..n]);
        let mut buf2 = [0u8; SERIALIZATION_BUFFER_SIZE];
        let n2 = got.serialize(&mut buf2);
        assert_eq!(
            &buf[..n],
            &buf2[..n2],
            "serialize after deserialize differs"
        );
        (got, buf[..n].to_vec())
    }

    fn ip(
        dollar_count: u8,
        open_brace_count: u8,
        quote_count: u8,
        string_type: u8,
    ) -> Interpolation {
        Interpolation {
            dollar_count,
            open_brace_count,
            quote_count,
            string_type,
        }
    }

    #[test]
    fn serialize_bytes() {
        let mut s = Scanner::new();
        assert_eq!(roundtrip(&s).1, [0, 0], "fresh scanner");

        s.quote_count = 5;
        s.interpolation_stack = vec![
            ip(1, 1, 1, STRING_REGULAR),
            ip(255, 0, 200, STRING_RAW),
            ip(2, 2, 1, STRING_VERBATIM | STRING_REGULAR),
        ];
        let (got, b) = roundtrip(&s);
        // quote_count, stack size, then 4 bytes per entry in struct order.
        let want = [5, 3, 1, 1, 1, 1, 255, 0, 200, 4, 2, 2, 1, 3];
        assert_eq!(b, want);
        assert!(
            got.quote_count == 5 && got.interpolation_stack == s.interpolation_stack,
            "state = {got:?}, want {s:?}"
        );
    }

    #[test]
    fn deserialize_empty() {
        let mut s = Scanner {
            quote_count: 4,
            interpolation_stack: vec![ip(1, 1, 1, 1)],
        };
        s.deserialize(&[]);
        assert!(
            s.quote_count == 0 && s.interpolation_stack.is_empty(),
            "state = {s:?}"
        );
    }

    #[test]
    fn serialize_limits() {
        // 255 entries is the most that fits: 255*4+2 = 1022.
        let mut s = Scanner::new();
        for i in 0..255 {
            s.interpolation_stack
                .push(ip(i as u8, 0, 1, STRING_REGULAR));
        }
        let (got, b) = roundtrip(&s);
        assert!(
            b.len() == 1022 && b[1] == 255 && got.interpolation_stack == s.interpolation_stack,
            "len = {}, count byte = {}",
            b.len(),
            b[1]
        );

        // One more and C writes nothing at all, so the state resets on reload.
        s.quote_count = 3;
        s.interpolation_stack.push(ip(1, 0, 1, STRING_REGULAR));
        assert_eq!(s.serialize(&mut [0u8; SERIALIZATION_BUFFER_SIZE]), 0);
    }

    #[test]
    fn iswspace_c_locale() {
        // What darwin libc answers in the C locale for -2..0x10FFFF.
        let want = [9, 10, 11, 12, 13, 32];
        let got: Vec<i32> = (-2..0x110000).filter(|&c| iswspace(c)).collect();
        assert_eq!(got, want, "iswspace true for {got:?}, want {want:?}");
    }
}

// gositter-rs-vjz.12: compiled when the runtime parser exists (Task 12).
#[cfg(any())]
mod parser_tests {
    // Parses through the scanner and checks the state left behind.
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn scan_state() {
        let mut p = gositter::Parser::new();
        p.set_language(super::super::language()).unwrap();
        for (src, want) in [
            (r#"var s = $"a{b}c";"#, "interpolated_string_expression"),
            (r#"var s = $@"a""{b}c";"#, "interpolated_string_expression"),
            (
                "var s = $$\"\"\"a{{b}}c\"\"\";",
                "interpolated_string_expression",
            ),
            ("var s = \"\"\"\"a\"\"\"b\"\"\"\";", "raw_string_literal"),
        ] {
            let tree = p.parse(src.as_bytes());
            let sexp = tree.root_node().to_string();
            assert!(
                !tree.root_node().has_error() && sexp.contains(want),
                "{src}: {sexp}"
            );
        }
    }
}

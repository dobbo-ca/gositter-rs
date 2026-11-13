//! Port of gositter `grammars/cpp/scanner.go` (tree-sitter-cpp v0.23.4 `src/scanner.c`): the
//! scanner state, `serialize`, `deserialize` and `reset`. `scan` waits for the runtime `Lexer`
//! (Task 12).

/// `MAX_DELIMITER_LENGTH`.
const MAX_DELIMITER_LENGTH: usize = 16;

/// `sizeof(wchar_t)`: 4 on every target of the C build. The serialized state is `delimiter_length`
/// of them, little-endian.
const SIZEOF_WCHAR: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner {
    delimiter_length: u8,
    /// `wchar_t` is a signed 32-bit int.
    delimiter: [i32; MAX_DELIMITER_LENGTH],
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// The state `new` returns.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        let mut size = 0;
        for c in &self.delimiter[..usize::from(self.delimiter_length)] {
            buf[size..size + SIZEOF_WCHAR].copy_from_slice(&c.to_le_bytes());
            size += SIZEOF_WCHAR;
        }
        size
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        // C asserts length % sizeof(wchar_t) == 0. The runtime only passes back what `serialize`
        // wrote, so at most 64 bytes in whole wchar_t units.
        self.delimiter_length = (buf.len() / SIZEOF_WCHAR) as u8;
        // Entries past the new length keep their old value, as with C's memcpy. Nothing reads them.
        for (d, c) in self
            .delimiter
            .iter_mut()
            .zip(buf.as_chunks::<SIZEOF_WCHAR>().0)
        {
            *d = i32::from_le_bytes(*c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn serialize(s: &Scanner) -> Vec<u8> {
        let mut buf = [0u8; crate::SERIALIZATION_BUFFER_SIZE];
        let n = s.serialize(&mut buf);
        buf[..n].to_vec()
    }

    // TestSerializeBytes
    #[test]
    fn serialize_bytes() {
        let mut s = Scanner::new();
        assert!(serialize(&s).is_empty(), "fresh scanner is not empty");

        // 'a', U+00E9, U+1F600, a real NUL and TS_DECODE_ERROR (-1), as 4-byte little-endian
        // wchar_t.
        s.delimiter_length = 5;
        s.delimiter[..5].copy_from_slice(&[b'a' as i32, 0xE9, 0x1F600, 0, -1]);
        let want: Vec<u8> = vec![
            b'a', 0, 0, 0, //
            0xE9, 0, 0, 0, //
            0x00, 0xF6, 0x01, 0, //
            0, 0, 0, 0, //
            0xFF, 0xFF, 0xFF, 0xFF,
        ];
        let b = serialize(&s);
        assert_eq!(b, want);

        // Dirty target: only the first delimiter_length entries count.
        let mut got = Scanner {
            delimiter_length: 16,
            delimiter: [b'z' as i32; MAX_DELIMITER_LENGTH],
        };
        got.deserialize(&b);
        assert!(
            got.delimiter_length == 5 && got.delimiter[4] == -1 && got.delimiter[2] == 0x1F600,
            "state = {got:?}"
        );
        assert_eq!(b, serialize(&got), "serialize after deserialize differs");
    }

    // TestDeserializeEmpty
    #[test]
    fn deserialize_empty() {
        let mut s = Scanner {
            delimiter_length: 2,
            ..Scanner::new()
        };
        (s.delimiter[0], s.delimiter[1]) = (b'x' as i32, b'y' as i32);
        s.deserialize(&[]);
        assert!(
            s.delimiter_length == 0 && serialize(&s).is_empty(),
            "state = {s:?}"
        );
        // C does not memcpy when length is 0, so the old chars stay.
        assert_eq!(s.delimiter[0], b'x' as i32, "delimiter was cleared");
    }

    // TestSerializeMax
    #[test]
    fn serialize_max() {
        let mut s = Scanner::new();
        s.delimiter_length = MAX_DELIMITER_LENGTH as u8;
        for (i, d) in s.delimiter.iter_mut().enumerate() {
            *d = b'A' as i32 + i as i32;
        }
        let b = serialize(&s);
        assert_eq!(b.len(), 64);
        let mut got = Scanner::new();
        got.deserialize(&b);
        assert_eq!(got, s);
    }

    // Reset: the state `new` returns.
    fn check_reset(s: &mut Scanner, what: &str) {
        s.reset();
        assert_eq!(*s, Scanner::new(), "{what}: after reset");
        assert_eq!(
            serialize(s),
            serialize(&Scanner::new()),
            "{what}: reset and new serialize differently"
        );
    }

    // The hand-made half of TestResetIsNew (reset_test.go); the parser half is in `parser_tests`.
    #[test]
    fn reset_hand_made() {
        let mut s = Scanner::new();
        s.delimiter_length = 3;
        (s.delimiter[0], s.delimiter[15]) = (b'a' as i32, b'z' as i32);
        check_reset(&mut s, "hand-made");
    }
}

// gositter-rs-vjz.12: compiled when the runtime parser exists (Task 12).
#[cfg(any())]
mod parser_tests {
    use super::tests::serialize;
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    // Raw strings through the parser: the scanner is wired in and its tokens land where the C
    // grammar puts them. TestRawStrings (scanner_test.go)
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn raw_strings() {
        for (src, want) in [
            (
                r#"auto s = R"x(a)y)x";"#,
                "(raw_string_literal delimiter: (raw_string_delimiter) (raw_string_content) (raw_string_delimiter))",
            ),
            (
                r#"auto s = R"(a"b)";"#,
                "(raw_string_literal (raw_string_content))",
            ),
            // 17 chars: over MAX_DELIMITER_LENGTH, no delimiter token.
            (
                r#"auto s = R"abcdefghijklmnopq(a)abcdefghijklmnopq";"#,
                "ERROR",
            ),
            // Unterminated: content runs to EOF.
            (r#"auto s = R"x(abc"#, "(raw_string_content)"),
        ] {
            let mut p = gositter::Parser::new();
            p.set_language(super::super::language());
            let tree = p.parse(src.as_bytes());
            let got = tree.root_node().to_string();
            assert!(got.contains(want), "{src}\n got {got}\nwant {want}");
        }
    }

    const RESET_INPUTS: [&str; 9] = [
        "",
        "int a;\n",
        "auto s = R\"x(body)x\";\n",
        "auto s = R\"(body)\";\n",
        "auto s = R\"abc(body without an end",
        "auto s = R\"abcdefghijklmnopq(x)\";\n",
        "auto s = R\"ab cd(x)\";\n",
        "auto s = R\"x(a)y\" R\"y(b",
        "auto s = LR\"d(\n\n",
    ];

    // The Rust form of `language.ExternalScanner.New = ...`: a spy wraps the scanner the parser
    // makes and keeps a handle to it. `Language: Clone`, and the constructor is a plain field.
    thread_local! {
        static KEPT: RefCell<Option<Rc<RefCell<Scanner>>>> = const { RefCell::new(None) };
    }

    struct Spy(Rc<RefCell<Scanner>>);

    impl gositter::ExternalScanner for Spy {
        fn scan(&mut self, l: &mut gositter::Lexer, valid: &[bool]) -> bool {
            self.0.borrow_mut().scan(l, valid)
        }
        fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
            self.0.borrow().serialize(buf)
        }
        fn deserialize(&mut self, buf: &[u8]) {
            self.0.borrow_mut().deserialize(buf)
        }
        fn reset(&mut self) -> bool {
            self.0.borrow_mut().reset();
            true
        }
    }

    fn spy() -> Box<dyn gositter::ExternalScanner> {
        let s = Rc::new(RefCell::new(Scanner::new()));
        KEPT.set(Some(s.clone()));
        Box::new(Spy(s))
    }

    fn spy_language() -> gositter::Language {
        let mut language = super::super::language().clone();
        language.external_scanner_new = spy;
        language
    }

    fn check_reset(s: &mut Scanner, what: &str) {
        s.reset();
        assert_eq!(*s, Scanner::new(), "{what}: after reset");
        assert_eq!(
            serialize(s),
            serialize(&Scanner::new()),
            "{what}: reset and new serialize differently"
        );
    }

    // TestResetIsNew (reset_test.go)
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn reset_is_new() {
        let language = spy_language();
        let mut dirty = 0;
        let mut p = gositter::Parser::new();
        p.set_language(&language);
        for input in RESET_INPUTS {
            for budget in [0u32, 1] {
                p.reset();
                p.parse_budget(input.as_bytes(), budget);
                let kept = KEPT.with_borrow(Clone::clone).expect("no scanner made");
                if *kept.borrow() != Scanner::new() {
                    dirty += 1;
                }
                check_reset(&mut kept.borrow_mut(), input.trim());
                // Reset must hold after the scanner is reused too.
                p.reset();
                p.parse_budget(input.as_bytes(), budget);
            }
        }
        assert_ne!(
            dirty, 0,
            "no input left the scanner dirty, the test checks nothing"
        );
    }

    // B parsed after A on one parser has the tree of B on a fresh parser.
    // TestReusedScannerSameTree (reset_test.go)
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn reused_scanner_same_tree() {
        let language = super::super::language();
        let mut reused = gositter::Parser::new();
        reused.set_language(language);
        for a in RESET_INPUTS {
            for b in RESET_INPUTS {
                reused.parse(a.as_bytes());
                let got = reused.parse(b.as_bytes()).root_node().to_string();
                let mut fresh = gositter::Parser::new();
                fresh.set_language(language);
                let want = fresh.parse(b.as_bytes()).root_node().to_string();
                assert_eq!(got, want, "{b:?} after {a:?}");
            }
        }
    }
}

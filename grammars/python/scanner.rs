//! Port of gositter `grammars/python/scanner.go` (tree-sitter-python v0.25.0 `src/scanner.c`): the
//! scanner state, `serialize`, `deserialize` and `reset`. `scan` waits for the runtime `Lexer`
//! (Task 12).

// enum Flags. A delimiter is one byte of these. BACK_QUOTE (1 << 2) and BYTES (1 << 6) are read
// only by `scan`, so they arrive with it.
const FLAG_SINGLE_QUOTE: u8 = 1 << 0;
const FLAG_DOUBLE_QUOTE: u8 = 1 << 1;
const FLAG_RAW: u8 = 1 << 3;
const FLAG_FORMAT: u8 = 1 << 4;
const FLAG_TRIPLE: u8 = 1 << 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scanner {
    indents: Vec<u16>,
    delimiters: Vec<u8>,
    inside_interpolated_string: bool,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    pub fn new() -> Self {
        let mut s = Scanner {
            indents: Vec::new(),
            delimiters: Vec::new(),
            inside_interpolated_string: false,
        };
        s.deserialize(&[]);
        s
    }

    /// The state `new` returns, with the buffers kept.
    pub fn reset(&mut self) {
        self.inside_interpolated_string = false;
        self.deserialize(&[]);
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        buf[0] = u8::from(self.inside_interpolated_string);
        let delimiter_count = self.delimiters.len().min(255);
        buf[1] = delimiter_count as u8;
        let mut size = 2;

        buf[size..size + delimiter_count].copy_from_slice(&self.delimiters[..delimiter_count]);
        size += delimiter_count;

        // C tests only size < 1024 and then writes two bytes. With an odd delimiter count and 511+
        // indents (511 nested blocks, then a string start) it writes one byte past the buffer and
        // returns 1025; the runtime's ts_assert(length <= 1024) then aborts the process. There is
        // no C tree to match, so stop one pair earlier.
        for indent in self.indents.iter().skip(1) {
            if size + 1 >= buf.len() {
                break;
            }
            buf[size..size + 2].copy_from_slice(&indent.to_le_bytes());
            size += 2;
        }

        size
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        self.delimiters.clear();
        self.indents.clear();
        self.indents.push(0);
        // inside_interpolated_string is not reset for an empty buffer; C leaves it.

        if !buf.is_empty() {
            self.inside_interpolated_string = buf[0] != 0;

            let delimiter_count = usize::from(buf[1]);
            let mut size = 2;
            self.delimiters
                .extend_from_slice(&buf[size..size + delimiter_count]);
            size += delimiter_count;

            self.indents.extend(
                buf[size..]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| u16::from_le_bytes(*c)),
            );
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

    fn round_trip(s: &Scanner) -> (Scanner, Vec<u8>) {
        let b = serialize(s);
        // Dirty target: deserialize must replace all of it.
        let mut got = Scanner {
            indents: vec![9, 9],
            delimiters: vec![0xFF],
            inside_interpolated_string: true,
        };
        got.deserialize(&b);
        let b2 = serialize(&got);
        assert_eq!(b, b2, "serialize after deserialize differs");
        (got, b)
    }

    // TestSerializeBytes
    #[test]
    fn serialize_bytes() {
        let mut s = Scanner::new();
        assert_eq!(serialize(&s), [0, 0], "fresh scanner");

        s.indents = vec![0, 4, 8, 0x1234];
        s.delimiters = vec![
            FLAG_DOUBLE_QUOTE | FLAG_FORMAT,
            FLAG_SINGLE_QUOTE | FLAG_TRIPLE | FLAG_RAW,
        ];
        s.inside_interpolated_string = true;
        let (got, b) = round_trip(&s);
        // inside, count, delimiters, then indents[1:] little-endian.
        let want = [1, 2, 0x12, 0x29, 4, 0, 8, 0, 0x34, 0x12];
        assert_eq!(b, want);
        assert!(
            got.indents == s.indents
                && got.delimiters == s.delimiters
                && got.inside_interpolated_string,
            "state = {got:?}, want {s:?}"
        );
    }

    // TestDeserializeEmpty
    #[test]
    fn deserialize_empty() {
        let mut s = Scanner {
            indents: vec![0, 4],
            delimiters: vec![1],
            inside_interpolated_string: true,
        };
        s.deserialize(&[]);
        assert!(s.indents == [0] && s.delimiters.is_empty(), "state = {s:?}");
        // C does not touch inside_interpolated_string when length is 0.
        assert!(
            s.inside_interpolated_string,
            "inside_interpolated_string was reset"
        );
    }

    // TestSerializeLimits
    #[test]
    fn serialize_limits() {
        // 255 delimiters at most; the rest is dropped.
        let mut s = Scanner::new();
        s.delimiters = vec![FLAG_DOUBLE_QUOTE; 300];
        let (got, b) = round_trip(&s);
        assert!(
            b[1] == 255 && b.len() == 2 + 255 && got.delimiters.len() == 255,
            "len = {}, count byte = {}",
            b.len(),
            b[1]
        );

        // Even header: C fills the buffer exactly, 511 indents.
        let mut s = Scanner::new();
        s.indents.extend(1..=2000u16);
        let (got, b) = round_trip(&s);
        assert!(
            b.len() == 1024 && got.indents.len() == 512 && got.indents[511] == 511,
            "len = {}, indents = {}",
            b.len(),
            got.indents.len()
        );

        // Odd header: C overruns by one byte here. The port stops at 1023.
        s.delimiters = vec![FLAG_DOUBLE_QUOTE];
        let (_, b) = round_trip(&s);
        assert_eq!(b.len(), 1023);
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
    // inside_interpolated_string is a field deserialize(&[]) leaves alone.
    #[test]
    fn reset_hand_made() {
        let mut s = Scanner {
            indents: vec![0, 4, 8],
            delimiters: vec![FLAG_DOUBLE_QUOTE | FLAG_FORMAT, FLAG_SINGLE_QUOTE],
            inside_interpolated_string: true,
        };
        check_reset(&mut s, "hand-made");
    }
}

// gositter-rs-vjz.12: needs impl ExternalScanner for Scanner (Task 12)
#[cfg(any())]
mod reset_tests {
    use super::tests::serialize;
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    const RESET_INPUTS: [&str; 12] = [
        "",
        "x = 1\n",
        "if a:\n    if b:\n        if c:\n            pass\n",
        "def f():\n    return f\"a{b}c{{d}}\"\n",
        "x = f\"a{f'b{c}'}d\"\n",
        "if a:\n    if b:\n        x = f\"a{b",
        "def f():\n    s = '''text\n  more",
        "class A:\n    def f(self):\n        if x:\n            y = rb\"raw",
        "x = (1,\n   2,\n",
        "if a:\n  if b:\n    f(\"\"\"doc {x}\n",
        "try:\n    pass\nexcept* E:\n    pass\n",
        "x = f\"{a!r:>{w}}\" + (\n",
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

    // TestResetIsNew (reset_test.go). Vec equality ignores capacity, so Go's `norm` is not needed.
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

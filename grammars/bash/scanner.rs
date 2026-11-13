//! Port of the scanner state of `grammars/bash/scanner.go` (tree-sitter-bash v0.25.1
//! `src/scanner.c`): `new`, `serialize`, `deserialize` and `reset`. `scan` and the helpers only it
//! uses (token enum, `isw*`, `advance_word`, ...) wait for the runtime `Lexer` (Task 12).

use crate::SERIALIZATION_BUFFER_SIZE;

/// Heredoc. The two strings are C `Array(char)`: lookahead is cut to its low byte on push and
/// read back as a char (`cchar::value`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heredoc {
    pub is_raw: bool,
    pub started: bool,
    pub allows_indent: bool,
    pub delimiter: Vec<u8>,
    pub current_leading_word: Vec<u8>,
}

impl Heredoc {
    /// `reset_heredoc`.
    fn reset(&mut self) {
        self.is_raw = false;
        self.started = false;
        self.allows_indent = false;
        self.delimiter.clear(); // reset_string
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scanner {
    pub last_glob_paren_depth: u8,
    pub ext_was_in_double_quote: bool,
    pub ext_saw_outside_quote: bool,
    pub heredocs: Vec<Heredoc>,
}

impl Scanner {
    /// `tree_sitter_bash_external_scanner_create`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `heredoc_new` + `array_push`.
    pub fn push_heredoc(&mut self, allows_indent: bool) {
        self.heredocs.push(Heredoc {
            allows_indent,
            ..Heredoc::default()
        });
    }

    pub fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
        buf[0] = self.last_glob_paren_depth;
        buf[1] = u8::from(self.ext_was_in_double_quote);
        buf[2] = u8::from(self.ext_saw_outside_quote);
        buf[3] = self.heredocs.len() as u8; // (char) of the count, wraps at 256
        let mut size = 4;

        for h in &self.heredocs {
            if size + 3 + 4 + h.delimiter.len() >= SERIALIZATION_BUFFER_SIZE {
                return 0;
            }

            buf[size] = u8::from(h.is_raw);
            buf[size + 1] = u8::from(h.started);
            buf[size + 2] = u8::from(h.allows_indent);
            size += 3;

            buf[size..size + 4].copy_from_slice(&(h.delimiter.len() as u32).to_le_bytes());
            size += 4;
            buf[size..size + h.delimiter.len()].copy_from_slice(&h.delimiter);
            size += h.delimiter.len();
        }
        size
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        if buf.is_empty() {
            // reset: C clears each heredoc and keeps the count.
            for h in &mut self.heredocs {
                h.reset();
            }
            return;
        }
        self.last_glob_paren_depth = buf[0];
        self.ext_was_in_double_quote = buf[1] != 0;
        self.ext_saw_outside_quote = buf[2] != 0;
        let heredoc_count = usize::from(buf[3]);
        let mut size = 4;
        // C never shrinks heredocs here: slots past heredoc_count stay as they are.
        while self.heredocs.len() < heredoc_count {
            self.push_heredoc(false);
        }
        for h in self.heredocs.iter_mut().take(heredoc_count) {
            h.is_raw = buf[size] != 0;
            h.started = buf[size + 1] != 0;
            h.allows_indent = buf[size + 2] != 0;
            size += 3;

            let n = u32::from_le_bytes([buf[size], buf[size + 1], buf[size + 2], buf[size + 3]])
                as usize;
            size += 4;
            h.delimiter.clear();
            h.delimiter.extend_from_slice(&buf[size..size + n]);
            size += n;
        }
    }

    /// The state `new` returns, with the heredoc slots kept.
    pub fn reset(&mut self) {
        let mut heredocs = std::mem::take(&mut self.heredocs);
        // Give back what a huge input grew.
        if heredocs.capacity() > 16
            || heredocs
                .iter()
                .any(|d| d.delimiter.capacity() > 1024 || d.current_leading_word.capacity() > 1024)
        {
            heredocs = Vec::new();
        } else {
            heredocs.clear();
        }
        *self = Scanner {
            heredocs,
            ..Scanner::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serialize(s: &Scanner) -> Vec<u8> {
        let mut buf = [0u8; SERIALIZATION_BUFFER_SIZE];
        let n = s.serialize(&mut buf);
        buf[..n].to_vec()
    }

    // scanner_test.go TestSerializeBytes
    #[test]
    fn serialize_bytes() {
        let mut s = Scanner::new();
        assert_eq!(serialize(&s), [0, 0, 0, 0], "fresh scanner");

        s.last_glob_paren_depth = 0x81;
        s.ext_saw_outside_quote = true;
        s.push_heredoc(true);
        s.heredocs[0].is_raw = true;
        s.heredocs[0].delimiter.extend_from_slice(&[b'E', 0xFF, 0]);
        s.push_heredoc(false);
        s.heredocs[1].started = true;
        let b = serialize(&s);
        // header, then per heredoc: is_raw, started, allows_indent, uint32 LE size, bytes.
        let want = [
            0x81, 0, 1, 2, 1, 0, 1, 3, 0, 0, 0, b'E', 0xFF, 0, 0, 1, 0, 0, 0, 0, 0,
        ];
        assert_eq!(b, want, "bytes");

        // Dirty target with one stale heredoc: every serialized field is replaced.
        let mut got = Scanner {
            last_glob_paren_depth: 7,
            ext_was_in_double_quote: true,
            ..Scanner::default()
        };
        got.push_heredoc(false);
        got.heredocs[0].started = true;
        got.heredocs[0]
            .delimiter
            .extend_from_slice(b"STALESTALE\x00");
        got.deserialize(&b);
        assert_eq!(serialize(&got), want, "after round trip");
        let h = &got.heredocs[0];
        assert!(
            got.last_glob_paren_depth == 0x81
                && !got.ext_was_in_double_quote
                && got.ext_saw_outside_quote
                && h.is_raw
                && !h.started
                && h.allows_indent
                && h.delimiter == [b'E', 0xFF, 0],
            "state = {got:?}"
        );
    }

    // C resets each heredoc on an empty state but keeps heredocs.size, and a state with fewer
    // heredocs does not shrink the array either.
    // scanner_test.go TestDeserializeKeepsCount
    #[test]
    fn deserialize_keeps_count() {
        let mut s = Scanner {
            last_glob_paren_depth: 3,
            ext_was_in_double_quote: true,
            ..Scanner::default()
        };
        for _ in 0..3 {
            s.push_heredoc(true);
            let h = s.heredocs.last_mut().unwrap();
            h.is_raw = true;
            h.started = true;
            h.delimiter.extend_from_slice(&[b'X', 0]);
        }
        let one = [0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, b'Y', 0];
        s.deserialize(&one);
        assert!(
            s.heredocs.len() == 3
                && s.heredocs[0].delimiter == b"Y\x00"
                && s.heredocs[2].delimiter == b"X\x00",
            "state = {s:?}"
        );

        s.deserialize(&[]);
        let want = [
            0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert_eq!(serialize(&s), want, "after reset");
    }

    // scanner_test.go TestSerializeLimit
    #[test]
    fn serialize_limit() {
        // 4 + 3 + 4 + len must stay below 1024, else C returns 0.
        for (n, want) in [(1012usize, 1023usize), (1013, 0), (5000, 0)] {
            let mut s = Scanner::new();
            s.push_heredoc(false);
            s.heredocs[0].delimiter = vec![b'D'; n];
            let b = serialize(&s);
            assert_eq!(b.len(), want, "delimiter {n}: size");
            if want > 0 {
                let mut got = Scanner::new();
                got.deserialize(&b);
                assert_eq!(serialize(&got), b, "delimiter {n}: round trip differs");
            }
        }
        // 147 empty heredocs: 4 + 146*7 = 1026 would not fit.
        let mut s = Scanner::new();
        for _ in 0..145 {
            s.push_heredoc(false);
        }
        assert_eq!(serialize(&s).len(), 4 + 145 * 7, "145 heredocs");
        s.push_heredoc(false);
        assert_eq!(serialize(&s).len(), 0, "146 heredocs");
    }

    // reset_test.go checkReset. Capacity is the only allowed difference, and `==` ignores it.
    pub(super) fn check_reset(s: &mut Scanner, what: &str) {
        s.reset();
        assert_eq!(*s, Scanner::new(), "{what}: after reset, new gives");
        assert_eq!(
            serialize(s),
            serialize(&Scanner::new()),
            "{what}: reset and new serialize differently"
        );
    }

    // reset_test.go TestResetIsNew, the hand-made half (no parser needed)
    #[test]
    fn reset_hand_made() {
        // Fields deserialize(&[]) leaves alone.
        let mut s = Scanner {
            last_glob_paren_depth: 3,
            ext_was_in_double_quote: true,
            ext_saw_outside_quote: true,
            ..Scanner::default()
        };
        s.push_heredoc(true);
        s.heredocs[0].is_raw = true;
        s.heredocs[0].started = true;
        s.heredocs[0].delimiter = b"EOF".to_vec();
        s.heredocs[0].current_leading_word = b"lead".to_vec();
        s.push_heredoc(false);
        s.heredocs[1].delimiter = b"X".to_vec();
        check_reset(&mut s, "hand-made");
    }
}

#[cfg(test)]
mod parser_tests {
    use crate::cchar;

    /// The grammar's generated language() (grammars plan Task 10, bead gositter-rs-vjz.10). The tests below are ignored until then.
    fn language() -> &'static gositter::Language {
        unimplemented!("gositter-rs-vjz.10: generated language()")
    }

    // The delimiter is compared as C chars against lookahead, so the two non-ASCII cases swap
    // where char is unsigned.
    // scanner_test.go TestHeredocEndChar
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn heredoc_end_char() {
        let mut p = gositter::Parser::new();
        p.set_language(language()).unwrap();
        for (src, want_err) in [
            (&b"cat <<E\nx\nE\n"[..], false),
            // signed: 0xFF is -1, as is an invalid byte
            (&b"cat <<\xff\nx\n\xff\n"[..], cchar::UNSIGNED),
            // signed: U+00E9 is cut to 0xE9 = -23, never matches
            (&b"cat <<\xc3\xa9\nx\n\xc3\xa9\n"[..], !cchar::UNSIGNED),
        ] {
            let tree = p.parse(src).unwrap();
            assert_eq!(
                tree.root_node().has_error(),
                want_err,
                "{src:?}: has_error\n{}",
                tree.root_node().to_sexp()
            );
        }
    }
}

// gositter-rs-vjz.12: needs impl ExternalScanner for Scanner (Task 12)
#[cfg(any())]
mod reset_tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    const RESET_INPUTS: [&str; 12] = [
        "",
        "echo hi\n",
        "cat <<EOF\nbody\nEOF\n",
        "cat <<-A <<'B'\n\tone\n\tA\ntwo\nB\n",
        "cat <<EOF\nbody $(echo <<X\nin\nX\n) more\nEOF\n",
        "cat <<EOF\nbody without an end",
        "cat <<A <<B <<C\nx\nA\n",
        "echo ${x:-$(cat <<E\n",
        "echo \"$(( 1 + \n",
        "if [[ $x =~ ^a ]]; then echo 'unclosed\n",
        "echo ${a[@]#*} <<<\"s\"\nfoo=bar\n",
        "cat <<EOF >out; echo $(\n",
    ];

    thread_local! {
        // The scanner the parser made, as language.ExternalScanner.New captures it in Go.
        static KEPT: RefCell<Option<Rc<RefCell<Scanner>>>> = const { RefCell::new(None) };
    }

    /// Hands the parser a scanner the test keeps a second handle to.
    struct Shared(Rc<RefCell<Scanner>>);

    impl gositter::ExternalScanner for Shared {
        fn scan(&mut self, l: &mut gositter::Lexer, valid: &[bool]) -> bool {
            self.0.borrow_mut().scan(l, valid)
        }
        fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
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

    fn capture() -> Box<dyn gositter::ExternalScanner> {
        let rc = Rc::new(RefCell::new(Scanner::new()));
        KEPT.with(|k| *k.borrow_mut() = Some(rc.clone()));
        Box::new(Shared(rc))
    }

    // The test swaps the constructor on a clone of the language (`Language: Clone`).
    fn capturing_language() -> gositter::Language {
        let mut lang = super::super::language().clone();
        lang.external_scanner_new = capture;
        lang
    }

    fn kept() -> Rc<RefCell<Scanner>> {
        KEPT.with(|k| k.borrow().clone()).expect("no scanner made")
    }

    fn serialize(s: &Scanner) -> Vec<u8> {
        let mut buf = [0u8; SERIALIZATION_BUFFER_SIZE];
        let n = s.serialize(&mut buf);
        buf[..n].to_vec()
    }

    // reset_test.go TestResetIsNew
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn reset_is_new() {
        use super::tests::check_reset;
        let lang = capturing_language();
        let mut dirty = 0;
        let mut p = gositter::Parser::new();
        p.set_language(&lang).unwrap();
        for input in RESET_INPUTS {
            for budget in [0u32, 1] {
                p.reset();
                p.parse_budget(input.as_bytes(), budget);
                let k = kept();
                if *k.borrow() != Scanner::new() {
                    dirty += 1;
                }
                check_reset(&mut k.borrow_mut(), input.trim());
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
    // reset_test.go TestReusedScannerSameTree
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn reused_scanner_same_tree() {
        let mut reused = gositter::Parser::new();
        reused.set_language(super::super::language()).unwrap();
        for a in RESET_INPUTS {
            for b in RESET_INPUTS {
                reused.parse(a.as_bytes());
                let got = reused.parse(b.as_bytes()).root_node().to_string();
                let mut fresh = gositter::Parser::new();
                fresh.set_language(super::super::language()).unwrap();
                let want = fresh.parse(b.as_bytes()).root_node().to_string();
                assert_eq!(got, want, "{b:?} after {a:?}");
            }
        }
    }

    // A small parse after a huge one must not keep the huge buffers.
    // reset_test.go TestResetDropsBigBuffers
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn reset_drops_big_buffers() {
        for input in [
            "cat <<A; ".repeat(20000),
            format!("cat <<{}\nbody\n", "A".repeat(2000000)),
        ] {
            let lang = capturing_language();
            let mut p = gositter::Parser::new();
            p.set_language(&lang).unwrap();
            p.parse(input.as_bytes());
            p.parse(b"echo hi\n");
            let k = kept();
            let k = k.borrow();
            let held = k.heredocs.capacity()
                + k.heredocs
                    .iter()
                    .map(|h| h.delimiter.capacity() + h.current_leading_word.capacity())
                    .sum::<usize>();
            assert!(held <= 4096, "small parse kept {held} slots and bytes");
        }
    }
}

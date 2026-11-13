//! Port of gositter `grammars/hcl/scanner.go` (tree-sitter-hcl v1.2.0 `src/scanner.c`): the scanner
//! state, `serialize`, `deserialize` and `reset`. `scan` waits for the runtime `Lexer` (Task 12).

// enum ContextType. u32 because C serializes sizeof(ContextType) == 4.
const CTX_TEMPLATE_INTERPOLATION: u32 = 0;
const CTX_TEMPLATE_DIRECTIVE: u32 = 1;
const CTX_QUOTED_TEMPLATE: u32 = 2;
const CTX_HEREDOC_TEMPLATE: u32 = 3;

/// `CHAR_MAX`: 127, or 255 where char is unsigned (linux/arm64).
const CHAR_MAX: usize = crate::cchar::MAX as usize;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct Context {
    typ: u32,
    /// String `heredoc_identifier`. Only bytes: C stores each code point in a `char`, and
    /// `iswalnum` admits ASCII only.
    heredoc_identifier: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Scanner {
    /// `context_vec`.
    context_stack: Vec<Context>,
}

impl Scanner {
    pub fn new() -> Self {
        todo!()
    }

    /// `VEC_PUSH` with a fresh `string_new()` identifier; returns the new top.
    fn push(&mut self, typ: u32) -> &mut Context {
        let _ = typ;
        todo!()
    }

    /// The state `new` returns.
    pub fn reset(&mut self) {
        todo!()
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        let _ = (buf, CHAR_MAX);
        todo!()
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        let _ = buf;
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(context_stack: Vec<Context>) -> Scanner {
        Scanner { context_stack }
    }

    fn ctx(typ: u32, id: &str) -> Context {
        Context {
            typ,
            heredoc_identifier: id.as_bytes().to_vec(),
        }
    }

    pub(super) fn serialize(s: &Scanner) -> Vec<u8> {
        let mut buf = [0u8; crate::SERIALIZATION_BUFFER_SIZE];
        let n = s.serialize(&mut buf);
        buf[..n].to_vec()
    }

    // TestSerializeBytes
    #[test]
    fn serialize_bytes() {
        assert_eq!(
            serialize(&Scanner::new()),
            [0, 0, 0, 0],
            "fresh scanner is not four zero bytes"
        );

        let s = stack(vec![
            ctx(CTX_HEREDOC_TEMPLATE, "EOT"),
            ctx(CTX_TEMPLATE_INTERPOLATION, ""),
            ctx(CTX_QUOTED_TEMPLATE, ""),
            ctx(CTX_TEMPLATE_DIRECTIVE, ""),
        ]);
        // u32 count, then per context: u32 type, u32 length, identifier bytes.
        let want: Vec<u8> = vec![
            4, 0, 0, 0, //
            3, 0, 0, 0, 3, 0, 0, 0, b'E', b'O', b'T', //
            0, 0, 0, 0, 0, 0, 0, 0, //
            2, 0, 0, 0, 0, 0, 0, 0, //
            1, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut b = serialize(&s);
        assert_eq!(b, want);

        // Dirty target with a longer stale identifier: all of it must go.
        let mut got = stack(vec![ctx(CTX_QUOTED_TEMPLATE, "STALESTALE"), ctx(9, "")]);
        got.deserialize(&b);
        assert_eq!(b, serialize(&got), "serialize after deserialize differs");
        assert!(
            got.context_stack.len() == 4
                && got.context_stack[0].heredoc_identifier == b"EOT"
                && got.context_stack[3].typ == CTX_TEMPLATE_DIRECTIVE,
            "state = {:?}",
            got.context_stack
        );

        // The buffer belongs to the runtime; the scanner must hold a copy.
        b[12] = b'X';
        assert_eq!(
            got.context_stack[0].heredoc_identifier, b"EOT",
            "deserialize kept a reference to the buffer"
        );
    }

    // TestDeserializeEmpty
    #[test]
    fn deserialize_empty() {
        let mut s = stack(vec![ctx(CTX_HEREDOC_TEMPLATE, "EOT")]);
        s.deserialize(&[]);
        assert!(s.context_stack.is_empty(), "state = {:?}", s.context_stack);
        // A reused slot must not show the old identifier.
        let top = s.push(CTX_QUOTED_TEMPLATE);
        assert!(
            top.heredoc_identifier.is_empty(),
            "reused identifier = {:?}",
            top.heredoc_identifier
        );
    }

    // TestSerializeLimits
    #[test]
    fn serialize_limits() {
        let n = |count: usize, id: &str| {
            let mut s = Scanner::default();
            for _ in 0..count {
                s.push(CTX_HEREDOC_TEMPLATE).heredoc_identifier = id.as_bytes().to_vec();
            }
            serialize(&s).len()
        };
        // More than CHAR_MAX contexts, or an identifier over CHAR_MAX: C writes 0.
        assert_eq!(n(127, ""), 4 + 127 * 8, "127 contexts");
        assert_eq!(n(128, ""), 0, "128 contexts");
        let id127 = "a".repeat(127);
        assert_eq!(n(1, &id127), 4 + 8 + 127, "identifier of 127");
        let want128 = if CHAR_MAX == 255 {
            // unsigned char: 128 fits
            4 + 8 + 128
        } else {
            0
        };
        assert_eq!(n(1, &format!("{id127}a")), want128, "identifier of 128");
        // The size test is >=, so 1023 bytes is the most C ever returns:
        // 4 + 7*(8+127) = 949, then 8+66 gives 1023 and 8+67 gives 0.
        let fill = |last: usize| {
            let mut s = Scanner::default();
            for _ in 0..7 {
                s.push(CTX_HEREDOC_TEMPLATE).heredoc_identifier = id127.as_bytes().to_vec();
            }
            s.push(CTX_HEREDOC_TEMPLATE).heredoc_identifier = vec![b'b'; last];
            serialize(&s).len()
        };
        assert_eq!(fill(66), 1023, "fill(66)");
        assert_eq!(fill(67), 0, "fill(67)");
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
        s.push(CTX_HEREDOC_TEMPLATE).heredoc_identifier = b"EOF".to_vec();
        s.push(CTX_QUOTED_TEMPLATE);
        check_reset(&mut s, "hand-made");
    }
}

// gositter-rs-vjz.12: compiled when the runtime parser exists (Task 12).
#[cfg(any())]
mod parser_tests {
    use super::tests::serialize;
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    const RESET_INPUTS: [&str; 9] = [
        "",
        "a = 1\n",
        "a = \"x ${y} z\"\n",
        "a = <<EOF\nbody ${x}\nEOF\n",
        "a = <<-EOF\n  %{ if x }\n  b\n  %{ endif }\n  EOF\n",
        "a = <<EOF\nbody without an end",
        "a = \"x ${y + \"${z\n",
        "a = \"%{ for i in l }${i}%{ endfor }\"\nb = <<A\n${\n",
        "a = \"${ \"${ \"${ \"unclosed\n",
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

    // TestResetDropsBigBuffers (reset_test.go). Go counts the capacity of the kept slots and of
    // their identifiers; a Vec keeps no spare slots, so the Vec and its identifiers are counted.
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn reset_drops_big_buffers() {
        for input in [
            "a = \"${b(\"${".repeat(20000),
            format!("a = <<{}\nbody\n", "A".repeat(2_000_000)),
        ] {
            let language = spy_language();
            let mut p = gositter::Parser::new();
            p.set_language(&language);
            p.parse(input.as_bytes());
            p.parse(b"a = 1\n");
            let kept = KEPT.with_borrow(Clone::clone).expect("no scanner made");
            let kept = kept.borrow();
            let held = kept.context_stack.capacity()
                + kept
                    .context_stack
                    .iter()
                    .map(|c| c.heredoc_identifier.capacity())
                    .sum::<usize>();
            assert!(held <= 4096, "small parse kept {held} slots and bytes");
        }
    }
}

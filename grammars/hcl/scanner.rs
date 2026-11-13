//! Port of gositter `grammars/hcl/scanner.go` (tree-sitter-hcl v1.2.0 `src/scanner.c`): the scanner
//! state, `serialize`, `deserialize`, `reset` and `scan`.
//!
//! skip_comment is not ported: C defines it and never calls it.

use gositter::{Lexer, Symbol};

/// enum TokenType
#[derive(Clone, Copy)]
enum Tok {
    QuotedTemplateStart,
    QuotedTemplateEnd,
    TemplateLiteralChunk,
    TemplateInterpolationStart,
    TemplateInterpolationEnd,
    TemplateDirectiveStart,
    TemplateDirectiveEnd,
    HeredocIdentifier,
}

// enum ContextType. u32 because C serializes sizeof(ContextType) == 4.
const CTX_TEMPLATE_INTERPOLATION: u32 = 0;
const CTX_TEMPLATE_DIRECTIVE: u32 = 1;
const CTX_QUOTED_TEMPLATE: u32 = 2;
const CTX_HEREDOC_TEMPLATE: u32 = 3;

/// `CHAR_MAX`: 127, or 255 where char is unsigned (linux/arm64).
const CHAR_MAX: usize = crate::cchar::MAX as usize;

// The isw* functions are libc and locale dependent. The parser never calls setlocale, so the "C"
// locale applies; there darwin and glibc classify only ASCII. Checked on darwin/arm64 against libc
// for every value -2..0x2FFFF.

fn iswspace(c: i32) -> bool {
    c == b' ' as i32 || (b'\t' as i32..=b'\r' as i32).contains(&c)
}

fn iswalnum(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
        || (b'a' as i32..=b'z' as i32).contains(&c)
        || (b'A' as i32..=b'Z' as i32).contains(&c)
}

fn iswxdigit(c: i32) -> bool {
    (b'0' as i32..=b'9' as i32).contains(&c)
        || (b'a' as i32..=b'f' as i32).contains(&c)
        || (b'A' as i32..=b'F' as i32).contains(&c)
}

/// accept_inplace
fn accept_inplace(l: &mut Lexer<'_>, token: Tok) -> bool {
    l.result_symbol = token as Symbol;
    true
}

/// accept_and_advance
fn accept_and_advance(l: &mut Lexer<'_>, token: Tok) -> bool {
    l.advance(false);
    accept_inplace(l, token)
}

/// consume_wxdigit
fn consume_wxdigit(l: &mut Lexer<'_>) -> bool {
    l.advance(false);
    iswxdigit(l.lookahead)
}

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
        Self::default()
    }

    /// `VEC_PUSH` with a fresh `string_new()` identifier; returns the new top.
    fn push(&mut self, typ: u32) -> &mut Context {
        self.context_stack.push(Context {
            typ,
            heredoc_identifier: Vec::new(),
        });
        self.context_stack.last_mut().unwrap()
    }

    /// The state `new` returns.
    pub fn reset(&mut self) {
        // Go keeps the spare slots and identifier buffers and gives them back past 16 slots or
        // 1024 identifier bytes. A Vec drops its elements on clear, so only the slot count is left.
        self.context_stack.clear();
        // Give back what a huge input grew.
        if self.context_stack.capacity() > 16 {
            self.context_stack = Vec::new();
        }
    }

    pub fn serialize(&self, buf: &mut [u8; crate::SERIALIZATION_BUFFER_SIZE]) -> usize {
        if self.context_stack.len() > CHAR_MAX {
            return 0;
        }

        buf[..4].copy_from_slice(&(self.context_stack.len() as u32).to_le_bytes());
        let mut size = 4;
        for ctx in &self.context_stack {
            let id = &ctx.heredoc_identifier;
            if size + 4 + 4 + id.len() >= buf.len() {
                return 0;
            }
            if id.len() > CHAR_MAX {
                return 0;
            }
            buf[size..size + 4].copy_from_slice(&ctx.typ.to_le_bytes());
            size += 4;
            buf[size..size + 4].copy_from_slice(&(id.len() as u32).to_le_bytes());
            size += 4;
            buf[size..size + id.len()].copy_from_slice(id);
            size += id.len();
        }
        size
    }

    pub fn deserialize(&mut self, buf: &[u8]) {
        self.context_stack.clear();

        if buf.is_empty() {
            return;
        }

        let u32_at =
            |at: usize| u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
        let mut size = 0;
        let context_stack_size = u32_at(size);
        size += 4;
        for _ in 0..context_stack_size {
            let ctx = self.push(u32_at(size));
            size += 4;

            let heredoc_identifier_size = u32_at(size) as usize;
            size += 4;
            ctx.heredoc_identifier
                .extend_from_slice(&buf[size..size + heredoc_identifier_size]);
            size += heredoc_identifier_size;
        }
        // C asserts size == length here; true for anything `serialize` wrote.
    }

    /// VEC_POP
    fn pop(&mut self) {
        self.context_stack.pop();
    }

    /// in_context_type
    fn in_context_type(&self, typ: u32) -> bool {
        match self.context_stack.last() {
            None => false,
            Some(ctx) => ctx.typ == typ,
        }
    }

    /// in_quoted_context
    fn in_quoted_context(&self) -> bool {
        self.in_context_type(CTX_QUOTED_TEMPLATE)
    }

    /// in_heredoc_context
    fn in_heredoc_context(&self) -> bool {
        self.in_context_type(CTX_HEREDOC_TEMPLATE)
    }

    /// in_template_context
    fn in_template_context(&self) -> bool {
        self.in_quoted_context() || self.in_heredoc_context()
    }

    /// in_interpolation_context
    fn in_interpolation_context(&self) -> bool {
        self.in_context_type(CTX_TEMPLATE_INTERPOLATION)
    }

    /// in_directive_context
    fn in_directive_context(&self) -> bool {
        self.in_context_type(CTX_TEMPLATE_DIRECTIVE)
    }

    /// scan, tree_sitter_hcl_external_scanner_scan
    fn scan(&mut self, l: &mut Lexer<'_>, valid_symbols: &[bool]) -> bool {
        let valid = |t: Tok| valid_symbols[t as usize];
        let mut has_leading_whitespace_with_newline = false;
        while iswspace(l.lookahead) {
            if l.lookahead == b'\n' as i32 {
                has_leading_whitespace_with_newline = true;
            }
            l.advance(true);
        }
        if l.lookahead == 0 {
            return false;
        }
        // manage quoted context
        if valid(Tok::QuotedTemplateStart)
            && !self.in_quoted_context()
            && l.lookahead == b'"' as i32
        {
            self.push(CTX_QUOTED_TEMPLATE);
            return accept_and_advance(l, Tok::QuotedTemplateStart);
        }
        if valid(Tok::QuotedTemplateEnd) && self.in_quoted_context() && l.lookahead == b'"' as i32 {
            self.pop();
            return accept_and_advance(l, Tok::QuotedTemplateEnd);
        }

        // manage template interpolations
        if valid(Tok::TemplateInterpolationStart)
            && valid(Tok::TemplateLiteralChunk)
            && !self.in_interpolation_context()
            && l.lookahead == b'$' as i32
        {
            l.advance(false);
            if l.lookahead == b'{' as i32 {
                self.push(CTX_TEMPLATE_INTERPOLATION);
                return accept_and_advance(l, Tok::TemplateInterpolationStart);
            }
            // try to scan escape sequence
            if l.lookahead == b'$' as i32 {
                l.advance(false);
                if l.lookahead == b'{' as i32 {
                    // $${
                    return accept_and_advance(l, Tok::TemplateLiteralChunk);
                }
            }
            return accept_inplace(l, Tok::TemplateLiteralChunk);
        }
        if valid(Tok::TemplateInterpolationEnd)
            && self.in_interpolation_context()
            && l.lookahead == b'}' as i32
        {
            self.pop();
            return accept_and_advance(l, Tok::TemplateInterpolationEnd);
        }

        // manage template directives
        if valid(Tok::TemplateDirectiveStart)
            && valid(Tok::TemplateLiteralChunk)
            && !self.in_directive_context()
            && l.lookahead == b'%' as i32
        {
            l.advance(false);
            if l.lookahead == b'{' as i32 {
                self.push(CTX_TEMPLATE_DIRECTIVE);
                return accept_and_advance(l, Tok::TemplateDirectiveStart);
            }
            // try to scan escape sequence
            if l.lookahead == b'%' as i32 {
                l.advance(false);
                if l.lookahead == b'{' as i32 {
                    // %%{
                    return accept_and_advance(l, Tok::TemplateLiteralChunk);
                }
            }
            return accept_inplace(l, Tok::TemplateLiteralChunk);
        }
        if valid(Tok::TemplateDirectiveEnd)
            && self.in_directive_context()
            && l.lookahead == b'}' as i32
        {
            self.pop();
            return accept_and_advance(l, Tok::TemplateDirectiveEnd);
        }

        // manage heredoc context
        if valid(Tok::HeredocIdentifier) && !self.in_heredoc_context() {
            let ctx = self.push(CTX_HEREDOC_TEMPLATE);
            while iswalnum(l.lookahead) || l.lookahead == b'_' as i32 || l.lookahead == b'-' as i32
            {
                // C narrows to char here; every accepted code point is ASCII.
                ctx.heredoc_identifier.push(l.lookahead as u8);
                l.advance(false);
            }
            return accept_inplace(l, Tok::HeredocIdentifier);
        }
        if valid(Tok::HeredocIdentifier)
            && self.in_heredoc_context()
            && has_leading_whitespace_with_newline
        {
            let expected_identifier =
                &self.context_stack[self.context_stack.len() - 1].heredoc_identifier;

            for &b in expected_identifier {
                if l.lookahead == i32::from(b) {
                    l.advance(false);
                } else {
                    return accept_inplace(l, Tok::TemplateLiteralChunk);
                }
            }
            // check if the identifier is on a line of its own
            l.mark_end();
            while iswspace(l.lookahead) && l.lookahead != b'\n' as i32 {
                l.advance(false);
            }
            if l.lookahead == b'\n' as i32 {
                self.pop();
                return accept_inplace(l, Tok::HeredocIdentifier);
            }
            l.advance(false);
            l.mark_end();
            return accept_inplace(l, Tok::TemplateLiteralChunk);
        }
        // manage template literal chunks

        // handle template literal chunks in quoted contexts
        //
        // they may not contain newlines and may contain escape sequences
        if valid(Tok::TemplateLiteralChunk)
            && self.in_quoted_context()
            && l.lookahead == b'\\' as i32
        {
            l.advance(false);
            match u8::try_from(l.lookahead) {
                Ok(b'"' | b'n' | b'r' | b't' | b'\\') => {
                    return accept_and_advance(l, Tok::TemplateLiteralChunk);
                }
                Ok(b'u') => {
                    for _ in 0..4 {
                        if !consume_wxdigit(l) {
                            return false;
                        }
                    }
                    return accept_and_advance(l, Tok::TemplateLiteralChunk);
                }
                Ok(b'U') => {
                    for _ in 0..8 {
                        if !consume_wxdigit(l) {
                            return false;
                        }
                    }
                    return accept_and_advance(l, Tok::TemplateLiteralChunk);
                }
                _ => return false,
            }
        }

        // handle all other quoted template or string literal characters
        if valid(Tok::TemplateLiteralChunk) && self.in_template_context() {
            return accept_and_advance(l, Tok::TemplateLiteralChunk);
        }

        // probably not handled by the external scanner
        false
    }
}

impl gositter::ExternalScanner for Scanner {
    fn scan(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        Scanner::scan(self, lexer, valid_symbols)
    }

    fn serialize(&self, buf: &mut [u8; gositter::SERIALIZATION_BUFFER_SIZE]) -> usize {
        Scanner::serialize(self, buf)
    }

    fn deserialize(&mut self, buf: &[u8]) {
        Scanner::deserialize(self, buf)
    }

    fn reset(&mut self) -> bool {
        Scanner::reset(self);
        true
    }
}

#[allow(dead_code)] // used by the generated language() (Task 10)
pub(crate) fn new() -> Box<dyn gositter::ExternalScanner> {
    Box::new(Scanner::new())
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
    pub(super) fn check_reset(s: &mut Scanner, what: &str) {
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

#[cfg(test)]
mod parser_tests {
    use super::Scanner;
    use super::tests::check_reset;

    /// The grammar's generated language() (grammars plan Task 10, bead gositter-rs-vjz.10). The tests below are ignored until then.
    fn language() -> &'static gositter::Language {
        unimplemented!("gositter-rs-vjz.10: generated language()")
    }

    pub(super) const RESET_INPUTS: [&str; 9] = [
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

    // B parsed after A on one parser has the tree of B on a fresh parser.
    // TestReusedScannerSameTree (reset_test.go)
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn reused_scanner_same_tree() {
        let mut reused = gositter::Parser::new();
        reused.set_language(language()).unwrap();
        for a in RESET_INPUTS {
            for b in RESET_INPUTS {
                reused.parse(a.as_bytes());
                let got = reused.parse(b.as_bytes()).unwrap().root_node().to_sexp();
                let mut fresh = gositter::Parser::new();
                fresh.set_language(language()).unwrap();
                let want = fresh.parse(b.as_bytes()).unwrap().root_node().to_sexp();
                assert_eq!(got, want, "{b:?} after {a:?}");
            }
        }
    }

    /// The scanner the parser keeps for its next parse (gositter `KeptScanner`).
    fn kept(p: &gositter::Parser) -> &Scanner {
        let s: &dyn std::any::Any = p.kept_scanner().expect("kept scanner");
        s.downcast_ref::<Scanner>().expect("this grammar's scanner")
    }

    // TestResetIsNew (reset_test.go). Go's `kept` is the scanner the parser made; here it is
    // read back with kept_scanner, and a clone is reset since the borrow is shared.
    // Vec equality ignores capacity, so Go's `norm` is not needed.
    #[test]
    #[ignore = "gositter-rs-2f0.11"]
    fn reset_is_new() {
        let mut dirty = 0;
        let mut p = gositter::Parser::new();
        p.set_language(language()).unwrap();
        for input in RESET_INPUTS {
            for budget in [0u32, 1] {
                p.reset();
                p.parse_budget(input.as_bytes(), budget);
                let mut s = kept(&p).clone();
                if s != Scanner::new() {
                    dirty += 1;
                }
                check_reset(&mut s, input.trim());
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

    // TestResetDropsBigBuffers (reset_test.go). Go counts the capacity of the kept slots and of
    // their buffers; a Vec keeps no readable spare slots, so the Vec and the buffers of its
    // elements are counted.
    #[test]
    #[ignore = "gositter-rs-2f0.11"]
    fn reset_drops_big_buffers() {
        for input in [
            "a = \"${b(\"${".repeat(20000),
            format!("a = <<{}\nbody\n", "A".repeat(2_000_000)),
        ] {
            let mut p = gositter::Parser::new();
            p.set_language(language()).unwrap();
            p.parse(input.as_bytes());
            p.parse(b"a = 1\n");
            let k = kept(&p);
            let held = k.context_stack.capacity()
                + k.context_stack
                    .iter()
                    .map(|c| c.heredoc_identifier.capacity())
                    .sum::<usize>();
            assert!(held <= 4096, "small parse kept {held} slots and bytes");
        }
    }
}

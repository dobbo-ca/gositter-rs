//! Port of gositter `grammars/csharp/scanner.go` (tree-sitter-c-sharp v0.23.5 `src/scanner.c`):
//! state, serialize, deserialize and scan.

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
    // ' ', then \t \n \v \f \r.
    c == 32 || (9..=13).contains(&c)
}

impl Scanner {
    /// `newScanner`
    pub fn new() -> Self {
        Self::default()
    }

    /// `Serialize`
    pub fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
        if self.interpolation_stack.len() * 4 + 2 > SERIALIZATION_BUFFER_SIZE {
            return 0;
        }

        let mut size = 0;

        buf[size] = self.quote_count;
        size += 1;
        // At most 255 entries pass the test above, so the byte does not wrap.
        buf[size] = self.interpolation_stack.len() as u8;
        size += 1;

        for i in &self.interpolation_stack {
            buf[size..size + 4].copy_from_slice(&[
                i.dollar_count,
                i.open_brace_count,
                i.quote_count,
                i.string_type,
            ]);
            size += 4;
        }

        size
    }

    /// `Deserialize`
    pub fn deserialize(&mut self, buf: &[u8]) {
        self.quote_count = 0;
        self.interpolation_stack.clear();
        let mut size = 0;

        if !buf.is_empty() {
            self.quote_count = buf[size];
            size += 1;
            let n = buf[size] as usize;
            size += 1;

            for _ in 0..n {
                self.interpolation_stack.push(Interpolation {
                    dollar_count: buf[size],
                    open_brace_count: buf[size + 1],
                    quote_count: buf[size + 2],
                    string_type: buf[size + 3],
                });
                size += 4;
            }
        }
        // C: assert(size == length). Holds for every buffer serialize wrote.
    }
}

impl Interpolation {
    /// `is_regular`
    fn is_regular(&self) -> bool {
        self.string_type & STRING_REGULAR != 0
    }

    /// `is_verbatim`
    fn is_verbatim(&self) -> bool {
        self.string_type & STRING_VERBATIM != 0
    }

    /// `is_raw`
    fn is_raw(&self) -> bool {
        self.string_type & STRING_RAW != 0
    }
}

impl gositter::ExternalScanner for Scanner {
    /// `Scan`
    fn scan(&mut self, lexer: &mut gositter::Lexer<'_>, valid_symbols: &[bool]) -> bool {
        let v = |t: u16| valid_symbols[t as usize];
        // uint8 counters wrap at 256 as in C.
        let mut brace_advanced = 0u8;
        let mut quote_count = 0u8;
        let mut did_advance = false;

        // error recovery, gives better trees this way
        if v(tok::OPT_SEMI) && v(tok::INTERPOLATION_REGULAR_START) {
            return false;
        }

        if v(tok::OPT_SEMI) {
            lexer.result_symbol = tok::OPT_SEMI;
            if lexer.lookahead == ';' as i32 {
                lexer.advance(false);
            }
            return true;
        }

        if v(tok::RAW_STRING_START) {
            while iswspace(lexer.lookahead) {
                lexer.advance(true);
            }

            if lexer.lookahead == '"' as i32 {
                while lexer.lookahead == '"' as i32 {
                    lexer.advance(false);
                    quote_count = quote_count.wrapping_add(1);
                }

                if quote_count >= 3 {
                    lexer.result_symbol = tok::RAW_STRING_START;
                    self.quote_count = quote_count;
                    return true;
                }
            }
        }

        if v(tok::RAW_STRING_END) && lexer.lookahead == '"' as i32 {
            while lexer.lookahead == '"' as i32 {
                lexer.advance(false);
                quote_count = quote_count.wrapping_add(1);
            }

            if quote_count == self.quote_count {
                lexer.result_symbol = tok::RAW_STRING_END;
                self.quote_count = 0;
                return true;
            }

            did_advance = quote_count > 0;
        }

        if v(tok::RAW_STRING_CONTENT) {
            while lexer.lookahead != 0 {
                if lexer.lookahead == '"' as i32 {
                    lexer.mark_end();
                    quote_count = 0;

                    while lexer.lookahead == '"' as i32 {
                        lexer.advance(false);
                        quote_count = quote_count.wrapping_add(1);
                    }

                    if quote_count == self.quote_count {
                        lexer.result_symbol = tok::RAW_STRING_CONTENT;
                        return true;
                    }
                }
                lexer.advance(false);
                // C sets did_advance = true here; nothing reads it, this branch always returns.
            }
            lexer.mark_end();
            lexer.result_symbol = tok::RAW_STRING_CONTENT;
            return true;
        }

        if v(tok::INTERPOLATION_REGULAR_START)
            || v(tok::INTERPOLATION_VERBATIM_START)
            || v(tok::INTERPOLATION_RAW_START)
        {
            while iswspace(lexer.lookahead) {
                lexer.advance(true);
            }

            let mut dollar_advanced = 0u8;

            let mut is_verbatim = false;

            if lexer.lookahead == '@' as i32 {
                is_verbatim = true;
                lexer.advance(false);
            }

            // quote_count can be non-zero here: the raw string branches above
            // may have consumed quotes without returning.
            while lexer.lookahead == '$' as i32 && quote_count == 0 {
                lexer.advance(false);
                dollar_advanced = dollar_advanced.wrapping_add(1);
            }

            if dollar_advanced > 0
                && (lexer.lookahead == '"' as i32 || lexer.lookahead == '@' as i32)
            {
                lexer.result_symbol = tok::INTERPOLATION_REGULAR_START;
                let mut ip = Interpolation {
                    dollar_count: dollar_advanced,
                    ..Interpolation::default()
                };

                if is_verbatim || lexer.lookahead == '@' as i32 {
                    if lexer.lookahead == '@' as i32 {
                        lexer.advance(false);
                        is_verbatim = true;
                    }
                    lexer.result_symbol = tok::INTERPOLATION_VERBATIM_START;
                    ip.string_type = STRING_VERBATIM;
                }

                lexer.mark_end();
                lexer.advance(false);

                if lexer.lookahead == '"' as i32 && !is_verbatim {
                    lexer.advance(false);
                    if lexer.lookahead == '"' as i32 {
                        lexer.result_symbol = tok::INTERPOLATION_RAW_START;
                        ip.string_type |= STRING_RAW;
                        self.interpolation_stack.push(ip);
                    }
                    // If we find 1 or 3 quotes, we push an interpolation.
                    // If there's only two quotes, that's just an empty string
                } else {
                    ip.string_type |= STRING_REGULAR;
                    self.interpolation_stack.push(ip);
                }

                return true;
            }
        }

        if v(tok::INTERPOLATION_START_QUOTE) && !self.interpolation_stack.is_empty() {
            let cur = self.interpolation_stack.last_mut().unwrap();

            if cur.is_verbatim() || cur.is_regular() {
                if lexer.lookahead == '"' as i32 {
                    lexer.advance(false);
                    cur.quote_count = cur.quote_count.wrapping_add(1);
                }
            } else {
                while lexer.lookahead == '"' as i32 {
                    lexer.advance(false);
                    cur.quote_count = cur.quote_count.wrapping_add(1);
                }
            }

            lexer.result_symbol = tok::INTERPOLATION_START_QUOTE;
            return cur.quote_count > 0;
        }

        if v(tok::INTERPOLATION_END_QUOTE) && !self.interpolation_stack.is_empty() {
            let cur = self.interpolation_stack.last().unwrap();

            while lexer.lookahead == '"' as i32 {
                lexer.advance(false);
                quote_count = quote_count.wrapping_add(1);
            }

            if quote_count == cur.quote_count {
                lexer.result_symbol = tok::INTERPOLATION_END_QUOTE;
                self.interpolation_stack.pop();
                return true;
            }

            did_advance = quote_count > 0;
        }

        if v(tok::INTERPOLATION_OPEN_BRACE) && !self.interpolation_stack.is_empty() {
            let cur = self.interpolation_stack.last_mut().unwrap();

            while lexer.lookahead == '{' as i32 && brace_advanced < cur.dollar_count {
                lexer.advance(false);
                brace_advanced = brace_advanced.wrapping_add(1);
            }

            if brace_advanced > 0
                && brace_advanced == cur.dollar_count
                && (brace_advanced == 0 || lexer.lookahead != '{' as i32)
            {
                cur.open_brace_count = brace_advanced;
                lexer.result_symbol = tok::INTERPOLATION_OPEN_BRACE;
                return true;
            }
        }

        if v(tok::INTERPOLATION_CLOSE_BRACE) && !self.interpolation_stack.is_empty() {
            let mut brace_advanced = 0u8; // shadows the outer one, as in C
            let cur = self.interpolation_stack.last_mut().unwrap();

            while iswspace(lexer.lookahead) {
                lexer.advance(false);
            }

            while lexer.lookahead == '}' as i32 {
                lexer.advance(false);
                brace_advanced = brace_advanced.wrapping_add(1);

                if brace_advanced == cur.open_brace_count {
                    cur.open_brace_count = 0;
                    lexer.result_symbol = tok::INTERPOLATION_CLOSE_BRACE;
                    return true;
                }
            }

            return false;
        }

        if v(tok::INTERPOLATION_STRING_CONTENT) && !self.interpolation_stack.is_empty() {
            lexer.result_symbol = tok::INTERPOLATION_STRING_CONTENT;
            let cur = self.interpolation_stack.last().unwrap();

            while lexer.lookahead != 0 {
                // top-down approach, first see if it's raw
                if cur.is_raw() {
                    if lexer.lookahead == '"' as i32 {
                        lexer.mark_end();
                        lexer.advance(false);
                        if lexer.lookahead == '"' as i32 {
                            lexer.advance(false);
                            let mut quote_advanced = 2u8;
                            while lexer.lookahead == '"' as i32 {
                                quote_advanced = quote_advanced.wrapping_add(1);
                                lexer.advance(false);
                            }
                            if quote_advanced == cur.quote_count {
                                return did_advance;
                            }
                        }
                    }

                    if lexer.lookahead == '{' as i32 {
                        lexer.mark_end();

                        while lexer.lookahead == '{' as i32 && brace_advanced < cur.open_brace_count
                        {
                            lexer.advance(false);
                            brace_advanced = brace_advanced.wrapping_add(1);
                        }

                        if brace_advanced == cur.open_brace_count
                            && (brace_advanced == 0 || lexer.lookahead != '{' as i32)
                        {
                            return did_advance;
                        }
                    }
                } else if cur.is_verbatim() {
                    // then verbatim, since it could be verbatim + raw, but run the raw branch first
                    if lexer.lookahead == '"' as i32 {
                        lexer.mark_end();
                        lexer.advance(false);
                        if lexer.lookahead == '"' as i32 {
                            lexer.advance(false);
                            continue;
                        }
                        return did_advance;
                    }

                    if lexer.lookahead == '{' as i32 {
                        lexer.mark_end();

                        while lexer.lookahead == '{' as i32 && brace_advanced < cur.open_brace_count
                        {
                            lexer.advance(false);
                            brace_advanced = brace_advanced.wrapping_add(1);
                        }

                        if brace_advanced == cur.open_brace_count
                            && (brace_advanced == 0 || lexer.lookahead != '{' as i32)
                        {
                            return did_advance;
                        }
                    }
                } else if cur.is_regular() {
                    // finally regular
                    if lexer.lookahead == '\\' as i32
                        || lexer.lookahead == '\n' as i32
                        || lexer.lookahead == '"' as i32
                    {
                        lexer.mark_end();
                        return did_advance;
                    }

                    if lexer.lookahead == '{' as i32 {
                        lexer.mark_end();

                        while lexer.lookahead == '{' as i32 && brace_advanced < cur.open_brace_count
                        {
                            lexer.advance(false);
                            brace_advanced = brace_advanced.wrapping_add(1);
                        }

                        // in a brace, never collect more than open_brace_count
                        if brace_advanced == cur.open_brace_count
                            && (brace_advanced == 0 || lexer.lookahead != '{' as i32)
                        {
                            return did_advance;
                        }
                    }
                }

                if lexer.lookahead != '{' as i32 {
                    brace_advanced = 0;
                }
                lexer.advance(false);
                did_advance = true;
            }

            lexer.mark_end();
            return did_advance;
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

#[cfg(test)]
mod parser_tests {
    /// The grammar's generated language() (grammars plan Task 10, bead gositter-rs-vjz.10). The tests below are ignored until then.
    fn language() -> &'static gositter::Language {
        unimplemented!("gositter-rs-vjz.10: generated language()")
    }

    // Parses through the scanner and checks the state left behind.
    #[test]
    #[ignore = "gositter-rs-vjz.12"]
    fn scan_state() {
        let mut p = gositter::Parser::new();
        p.set_language(language()).unwrap();
        for (src, want) in [
            (r#"var s = $"a{b}c";"#, "interpolated_string_expression"),
            (r#"var s = $@"a""{b}c";"#, "interpolated_string_expression"),
            (
                "var s = $$\"\"\"a{{b}}c\"\"\";",
                "interpolated_string_expression",
            ),
            ("var s = \"\"\"\"a\"\"\"b\"\"\"\";", "raw_string_literal"),
        ] {
            let tree = p.parse(src.as_bytes()).unwrap();
            let sexp = tree.root_node().to_sexp();
            assert!(
                !tree.root_node().has_error() && sexp.contains(want),
                "{src}: {sexp}"
            );
        }
    }
}

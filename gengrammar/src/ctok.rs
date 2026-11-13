//! Tokenizer for the C subset tree-sitter writes into `parser.c` (port of `ctok.go`). Errors are
//! `Result`s; Go panics and recovers, and the message text is the contract.

use std::collections::HashMap;
use std::fmt;

/// One token. `Char` carries the value and the source text, `Str` the decoded bytes.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Eof,
    Ident(String),
    Num(i64),
    Char(i64, String),
    Str(Vec<u8>),
    Punct(&'static str),
}

/// A token and its byte offset in the source.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub pos: usize,
}

/// A generator error. The message is `parser.c:<line>: <msg>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenError(pub String);

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GenError {}

/// Go's `strconv.ParseInt(s, 0, 64)`: optional sign, `0x`/`0o`/`0b` prefixes, a leading `0` for
/// octal, and `_` between digits.
pub fn parse_int(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let (neg, b) = match b.first() {
        Some(b'+') => (false, &b[1..]),
        Some(b'-') => (true, &b[1..]),
        _ => (false, b),
    };
    if b.is_empty() {
        return None;
    }
    let (base, digits) = if b[0] == b'0' {
        match b.get(1).map(|c| c | 0x20) {
            Some(b'b') if b.len() >= 3 => (2u8, &b[2..]),
            Some(b'o') if b.len() >= 3 => (8, &b[2..]),
            Some(b'x') if b.len() >= 3 => (16, &b[2..]),
            _ => (8, &b[1..]),
        }
    } else {
        (10, b)
    };
    let mut n: u64 = 0;
    let mut underscores = false;
    for &c in digits {
        let d = match c {
            b'_' => {
                underscores = true;
                continue;
            }
            b'0'..=b'9' => c - b'0',
            _ if (c | 0x20).is_ascii_lowercase() => (c | 0x20) - b'a' + 10,
            _ => return None,
        };
        if d >= base {
            return None;
        }
        n = n.checked_mul(u64::from(base))?.checked_add(u64::from(d))?;
    }
    if underscores && !underscore_ok(b) {
        return None;
    }
    if neg {
        (n <= 1 << 63).then(|| (n as i64).wrapping_neg())
    } else {
        i64::try_from(n).ok()
    }
}

/// Go's `underscoreOK`: an underscore must separate digits or follow a base prefix.
fn underscore_ok(s: &[u8]) -> bool {
    let mut saw = b'^';
    let mut hex = false;
    let mut rest = s;
    if s.len() >= 2 && s[0] == b'0' && matches!(s[1] | 0x20, b'b' | b'o' | b'x') {
        saw = b'0';
        hex = s[1] | 0x20 == b'x';
        rest = &s[2..];
    }
    for &c in rest {
        if c.is_ascii_digit() || hex && (b'a'..=b'f').contains(&(c | 0x20)) {
            saw = b'0';
        } else if c == b'_' {
            if saw != b'0' {
                return false;
            }
            saw = b'_';
        } else if saw == b'_' {
            return false;
        } else {
            saw = b'!';
        }
    }
    saw != b'_'
}

/// Go's `strconv.ParseUint(digits, radix, bits)` for ASCII hex or octal digits and no sign.
fn parse_uint(digits: &[u8], radix: u32, max: u64) -> Option<u64> {
    if digits.is_empty() {
        return None;
    }
    let mut n: u64 = 0;
    for &c in digits {
        n = n * u64::from(radix) + u64::from(char::from(c).to_digit(radix)?);
        if n > max {
            return None;
        }
    }
    Some(n)
}

pub fn is_ident_byte(c: u8, first: bool) -> bool {
    c == b'_' || c.is_ascii_alphabetic() || !first && c.is_ascii_digit()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// The tokenizer; `#define NAME <integer>` lines land in `defines`.
pub struct Tokenizer {
    src: String,
    i: usize,
    pub defines: HashMap<String, i64>,
}

impl Tokenizer {
    pub fn new(src: &str) -> Self {
        Tokenizer {
            src: src.to_string(),
            i: 0,
            defines: HashMap::new(),
        }
    }

    /// An error at byte `pos`: `parser.c:<line>: <msg>`, line = 1 + the newlines before `pos`.
    pub fn error(&self, pos: usize, msg: impl fmt::Display) -> GenError {
        let before = &self.src.as_bytes()[..pos.min(self.src.len())];
        let line = 1 + before.iter().filter(|&&c| c == b'\n').count();
        GenError(format!("parser.c:{line}: {msg}"))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Token, GenError> {
        let s = self.src.as_bytes();
        while self.i < s.len() {
            let c = s[self.i];
            if matches!(c, b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
            } else if c == b'#' {
                let end = s[self.i..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap_or(s.len() - self.i);
                if end > 0 && s[self.i + end - 1] == b'\\' {
                    return Err(self.error(self.i, "preprocessor line continuation"));
                }
                let f: Vec<&str> = self.src[self.i..self.i + end].split_whitespace().collect();
                if f.len() == 3
                    && f[0] == "#define"
                    && let Some(v) = parse_int(f[2])
                {
                    self.defines.insert(f[1].to_string(), v);
                }
                self.i += end;
            } else if c == b'/' && s.get(self.i + 1) == Some(&b'/') {
                self.i += s[self.i..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap_or(s.len() - self.i);
            } else if c == b'/' && s.get(self.i + 1) == Some(&b'*') {
                match find(&s[self.i + 2..], b"*/") {
                    None => return Err(self.error(self.i, "unterminated comment")),
                    Some(end) => self.i += end + 4,
                }
            } else {
                return self.scan();
            }
        }
        Ok(Token {
            tok: Tok::Eof,
            pos: self.i,
        })
    }

    fn scan(&mut self) -> Result<Token, GenError> {
        let s = self.src.as_bytes();
        let start = self.i;
        let c = s[start];
        if is_ident_byte(c, true) {
            let mut j = start + 1;
            while j < s.len() && is_ident_byte(s[j], false) {
                j += 1;
            }
            self.i = j;
            return Ok(Token {
                tok: Tok::Ident(self.src[start..j].to_string()),
                pos: start,
            });
        }
        if c.is_ascii_digit() {
            let mut j = start + 1;
            while j < s.len() && is_ident_byte(s[j], false) {
                j += 1;
            }
            // A leading 0 is octal in C and in ParseInt alike.
            let Some(v) = parse_int(&self.src[start..j]) else {
                return Err(self.error(start, format!("bad number {:?}", &self.src[start..j])));
            };
            self.i = j;
            return Ok(Token {
                tok: Tok::Num(v),
                pos: start,
            });
        }
        if c == b'\'' {
            let mut j = start + 1;
            let v = if j < s.len() && s[j] == b'\\' {
                let mut b = Vec::new();
                j = self.escape(j, &mut b)?;
                if b.len() != 1 {
                    return Err(self.error(start, "multi-byte character literal"));
                }
                i64::from(b[0])
            } else if j < s.len() && s[j] < 0x80 && s[j] != b'\'' && s[j] != b'\n' {
                j += 1;
                i64::from(s[j - 1])
            } else {
                return Err(self.error(start, "bad character literal"));
            };
            if j >= s.len() || s[j] != b'\'' {
                return Err(self.error(start, "bad character literal"));
            }
            self.i = j + 1;
            return Ok(Token {
                tok: Tok::Char(v, self.src[start..=j].to_string()),
                pos: start,
            });
        }
        if c == b'"' {
            let mut j = start + 1;
            let mut b = Vec::new();
            loop {
                if j >= s.len() || s[j] == b'\n' {
                    return Err(self.error(start, "unterminated string"));
                }
                if s[j] == b'"' {
                    break;
                }
                if s[j] == b'\\' {
                    j = self.escape(j, &mut b)?;
                } else {
                    b.push(s[j]);
                    j += 1;
                }
            }
            self.i = j + 1;
            return Ok(Token {
                tok: Tok::Str(b),
                pos: start,
            });
        }
        if let Some(two) = s.get(start..start + 2) {
            let p = match two {
                b"==" => Some("=="),
                b"!=" => Some("!="),
                b"<=" => Some("<="),
                b">=" => Some(">="),
                b"&&" => Some("&&"),
                b"||" => Some("||"),
                b"->" => Some("->"),
                _ => None,
            };
            if let Some(p) = p {
                self.i = start + 2;
                return Ok(Token {
                    tok: Tok::Punct(p),
                    pos: start,
                });
            }
        }
        let p = match c {
            b'{' => "{",
            b'}' => "}",
            b'[' => "[",
            b']' => "]",
            b'(' => "(",
            b')' => ")",
            b'<' => "<",
            b'>' => ">",
            b'=' => "=",
            b'!' => "!",
            b',' => ",",
            b';' => ";",
            b':' => ":",
            b'.' => ".",
            b'&' => "&",
            b'*' => "*",
            b'-' => "-",
            _ => return Err(self.error(start, format!("unexpected byte {:?}", char::from(c)))),
        };
        self.i = start + 1;
        Ok(Token {
            tok: Tok::Punct(p),
            pos: start,
        })
    }

    /// Decodes the C escape sequence at `s[j] == '\\'`, appends its bytes to `b` and returns the
    /// position after it.
    fn escape(&self, j: usize, b: &mut Vec<u8>) -> Result<usize, GenError> {
        let s = self.src.as_bytes();
        if j + 1 >= s.len() {
            return Err(self.error(j, "bad escape"));
        }
        let c = s[j + 1];
        let j = j + 2;
        match c {
            b'a' => b.push(7),
            b'b' => b.push(8),
            b'e' => b.push(27),
            b'f' => b.push(12),
            b'n' => b.push(b'\n'),
            b'r' => b.push(b'\r'),
            b't' => b.push(b'\t'),
            b'v' => b.push(11),
            b'\\' | b'\'' | b'"' | b'?' => b.push(c),
            b'x' => {
                let mut k = j;
                while k < s.len() && s[k].is_ascii_hexdigit() {
                    k += 1;
                }
                let Some(v) = parse_uint(&s[j..k], 16, 0xFF) else {
                    return Err(self.error(j, "bad \\x escape"));
                };
                b.push(v as u8);
                return Ok(k);
            }
            b'u' | b'U' => {
                let n = if c == b'U' { 8 } else { 4 };
                let v = if j + n <= s.len() {
                    parse_uint(&s[j..j + n], 16, 0xFFFF_FFFF)
                } else {
                    None
                };
                let Some(ch) = v.and_then(|v| char::from_u32(v as u32)) else {
                    return Err(self.error(j, "bad \\u escape"));
                };
                b.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
                return Ok(j + n);
            }
            b'0'..=b'7' => {
                let mut k = j;
                while k < s.len() && k < j + 2 && (b'0'..=b'7').contains(&s[k]) {
                    k += 1;
                }
                let Some(v) = parse_uint(&s[j - 1..k], 8, 0xFF) else {
                    return Err(self.error(j, "bad octal escape"));
                };
                b.push(v as u8);
                return Ok(k);
            }
            _ => return Err(self.error(j, format!("unknown escape \\{}", char::from(c)))),
        }
        Ok(j)
    }
}

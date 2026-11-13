use gengrammar::ctok::{GenError, Tok, Tokenizer};

fn all(src: &str) -> Result<Vec<Tok>, GenError> {
    let mut t = Tokenizer::new(src);
    let mut v = Vec::new();
    loop {
        let k = t.next()?;
        if k.tok == Tok::Eof {
            return Ok(v);
        }
        v.push(k.tok);
    }
}

fn toks(src: &str) -> Vec<Tok> {
    all(src).unwrap()
}

fn err(src: &str) -> String {
    all(src).unwrap_err().0
}

/// The message is `parser.c:<line>: <msg>`; check the whole text.
fn rej(src: &str, line: usize, msg: &str) {
    assert_eq!(
        err(src),
        format!("parser.c:{line}: {msg}"),
        "source {src:?}"
    );
}

fn num(v: i64) -> Tok {
    Tok::Num(v)
}

fn id(s: &str) -> Tok {
    Tok::Ident(s.into())
}

fn chr(v: i64, s: &str) -> Tok {
    Tok::Char(v, s.into())
}

fn st(s: &[u8]) -> Tok {
    Tok::Str(s.to_vec())
}

#[test]
fn define_sets_the_define_and_other_directives_are_skipped() {
    let mut t = Tokenizer::new(
        "#define LANGUAGE_VERSION 14\n#define X foo\n#define Y 0x10\n#define Z 1 2\n#include <stdint.h>\n#ifdef Q\nint\n",
    );
    assert_eq!(t.next().unwrap().tok, id("int"));
    assert_eq!(t.next().unwrap().tok, Tok::Eof);
    assert_eq!(t.defines["LANGUAGE_VERSION"], 14);
    assert_eq!(t.defines["Y"], 16);
    assert!(!t.defines.contains_key("X"));
    assert!(!t.defines.contains_key("Z"));
    assert_eq!(t.defines.len(), 2);
}

#[test]
fn preprocessor_line_continuation() {
    rej("#define X 1 \\\n2\n", 1, "preprocessor line continuation");
    rej("\n#define X \\", 2, "preprocessor line continuation");
}

#[test]
fn comments_and_whitespace() {
    assert_eq!(
        toks("a // x /* y\n b /* c\n d */ e\t\r\nf // tail"),
        [id("a"), id("b"), id("e"), id("f")]
    );
    assert_eq!(toks("/**/ x /***/ y"), [id("x"), id("y")]);
}

#[test]
fn unterminated_comment() {
    rej("x\n/* x", 2, "unterminated comment");
    rej("/*/", 1, "unterminated comment");
}

#[test]
fn identifiers_and_positions() {
    let mut t = Tokenizer::new("  _a1 Bc");
    let a = t.next().unwrap();
    assert_eq!((a.tok, a.pos), (id("_a1"), 2));
    let b = t.next().unwrap();
    assert_eq!((b.tok, b.pos), (id("Bc"), 6));
    let e = t.next().unwrap();
    assert_eq!((e.tok, e.pos), (Tok::Eof, 8));
}

#[test]
fn numbers_in_every_base() {
    assert_eq!(
        toks("0x1f 017 0b11 10 0 0X1F 0o17 0B1 1_0 0x_f"),
        [
            num(31),
            num(15),
            num(3),
            num(10),
            num(0),
            num(31),
            num(15),
            num(1),
            num(10),
            num(15)
        ]
    );
    assert_eq!(toks("9223372036854775807"), [num(i64::MAX)]);
}

#[test]
fn bad_numbers() {
    rej("10u", 1, "bad number \"10u\"");
    rej("08", 1, "bad number \"08\"");
    rej("0x", 1, "bad number \"0x\"");
    rej("1__0", 1, "bad number \"1__0\"");
    rej("1_", 1, "bad number \"1_\"");
    rej(
        "9223372036854775808",
        1,
        "bad number \"9223372036854775808\"",
    );
    rej("1e5", 1, "bad number \"1e5\"");
}

#[test]
fn char_literals() {
    assert_eq!(
        toks(
            r"'a' '\n' '\x41' '\101' '\'' '\\' '\0' '\a' '\b' '\e' '\f' '\r' '\t' '\v' '\?' 'A' '\U00000042'"
        ),
        [
            chr(97, "'a'"),
            chr(10, r"'\n'"),
            chr(65, r"'\x41'"),
            chr(65, r"'\101'"),
            chr(39, r"'\''"),
            chr(92, r"'\\'"),
            chr(0, r"'\0'"),
            chr(7, r"'\a'"),
            chr(8, r"'\b'"),
            chr(27, r"'\e'"),
            chr(12, r"'\f'"),
            chr(13, r"'\r'"),
            chr(9, r"'\t'"),
            chr(11, r"'\v'"),
            chr(63, r"'\?'"),
            chr(65, r"'A'"),
            chr(66, r"'\U00000042'"),
        ]
    );
    assert_eq!(toks("'\"' '\\\"'"), [chr(34, "'\"'"), chr(34, "'\\\"'")]);
    assert_eq!(
        toks(r"'\377' '\xff'"),
        [chr(255, r"'\377'"), chr(255, r"'\xff'")]
    );
}

#[test]
fn bad_char_literals() {
    rej("'ab'", 1, "bad character literal");
    rej("''", 1, "bad character literal");
    rej("'\n'", 1, "bad character literal");
    rej("'a", 1, "bad character literal");
    rej("'", 1, "bad character literal");
    // A raw non-ASCII byte is not a char literal; the escaped form is a multi-byte one.
    rej("'é'", 1, "bad character literal");
    rej(r"'\u00e9'", 1, "multi-byte character literal");
    rej(r"'\400'", 1, "bad octal escape");
    rej(r"'\q'", 1, r"unknown escape \q");
    rej(r"'\x'", 1, r"bad \x escape");
    rej(r"'\x100'", 1, r"bad \x escape");
    rej(r"'\", 1, "bad escape");
}

#[test]
fn strings() {
    assert_eq!(toks(r#""\"a\?""#), [st(b"\"a?")]);
    assert_eq!(toks(r#""\0""#), [st(&[0])]);
    assert_eq!(
        toks(r#""é" "\xc3\xa9" "\u00e9" "\n\\""#),
        [
            st(&[0xC3, 0xA9]),
            st(&[0xC3, 0xA9]),
            st(&[0xC3, 0xA9]),
            st(b"\n\\")
        ]
    );
    assert_eq!(toks(r#""" "a""b""#), [st(b""), st(b"a"), st(b"b")]);
    assert_eq!(toks(r#""\101\0017""#), [st(&[65, 1, b'7'])]);
}

#[test]
fn bad_strings() {
    rej("\"a\n\"", 1, "unterminated string");
    rej("x\n\"abc", 2, "unterminated string");
    rej(r#""\ud800""#, 1, r"bad \u escape");
    rej(r#""\U00110000""#, 1, r"bad \u escape");
    rej(r#""\u12""#, 1, r"bad \u escape");
    rej(r#""\u12g4""#, 1, r"bad \u escape");
    rej(r#""\q""#, 1, r"unknown escape \q");
}

#[test]
fn punctuation() {
    let want = [
        "==", "!=", "<=", ">=", "&&", "||", "->", "{", "}", "[", "]", "(", ")", "<", ">", "=", "!",
        ",", ";", ":", ".", "&", "*", "-",
    ];
    let src = want.join(" ");
    assert_eq!(toks(&src), want.map(Tok::Punct));
    assert_eq!(toks("a==b"), [id("a"), Tok::Punct("=="), id("b")]);
    assert_eq!(toks("=!"), [Tok::Punct("="), Tok::Punct("!")]);
    assert_eq!(toks("&&&"), [Tok::Punct("&&"), Tok::Punct("&")]);
}

#[test]
fn unexpected_bytes() {
    rej("+", 1, "unexpected byte '+'");
    rej("a\n b\n @", 3, "unexpected byte '@'");
    rej("/ x", 1, "unexpected byte '/'");
}

#[test]
fn errors_name_their_line() {
    let e = err("a\n\n 'ab'");
    assert!(e.starts_with("parser.c:3: "), "{e}");
    let e = err("a\n\n\"x\ny\"");
    assert!(e.starts_with("parser.c:3: "), "{e}");
}

#[test]
fn eof_repeats() {
    let mut t = Tokenizer::new("");
    assert_eq!(t.next().unwrap().tok, Tok::Eof);
    assert_eq!(t.next().unwrap().tok, Tok::Eof);
}

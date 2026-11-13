//! Parser of the C subset of `parser.c` into a [`Grammar`] (port of `parse.go`).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Display;

use crate::cond::{CmpOp, Cond, Operand};
use crate::ctok::{GenError, Tok, Token, Tokenizer, is_ident_byte};

type R<T> = Result<T, GenError>;

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub names: Vec<String>,
    pub vals: Vec<i64>,
}

/// One static array, flattened row-major. Struct elements take one value per field.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub name: String,
    pub ctype: String,
    /// As declared; 0 is an open dimension.
    pub dims: Vec<usize>,
    pub fields: Option<&'static [&'static str]>,
    pub vals: Vec<i64>,
    /// ctype `char*` only.
    pub strs: Vec<String>,
}

#[allow(clippy::len_without_is_empty)]
impl Table {
    pub fn width(&self) -> usize {
        let w = self.fields.map_or(0, |f| f.len()).max(1);
        self.dims[1..].iter().fold(w, |w, &d| w.wrapping_mul(d))
    }

    pub fn len(&self) -> usize {
        if self.ctype == "char*" {
            self.strs.len()
        } else {
            self.vals.len() / self.width()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    If { cond: Cond, skip: bool, target: u16 },
    Map(Vec<(u16, u16)>),
    Accept(String),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexCase {
    pub state: u16,
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexFunc {
    pub cname: String,
    pub cases: Vec<LexCase>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Grammar {
    /// The `<name>` of `tree_sitter_<name>`.
    pub name: String,
    pub defines: HashMap<String, i64>,
    /// Enum members and builtin symbols.
    pub consts: HashMap<String, i64>,
    pub enums: Vec<EnumDecl>,
    pub tables: HashMap<String, Table>,
    /// Table names in source order.
    pub order: Vec<String>,
    /// `ts_parse_actions`, 5 values per slot.
    pub actions: Vec<i64>,
    pub lex_fns: Vec<LexFunc>,
    /// The `TSLanguage` initialiser, nested keys joined with `.`.
    pub lang: BTreeMap<String, Vec<Token>>,
}

/// Struct layouts from `parser.h`, in field order.
fn struct_fields(ctype: &str) -> Option<&'static [&'static str]> {
    Some(match ctype {
        "TSSymbolMetadata" => &["visible", "named", "supertype"],
        "TSFieldMapSlice" | "TSMapSlice" => &["index", "length"],
        "TSFieldMapEntry" => &["field_id", "child_index", "inherited"],
        "TSLexMode" => &["lex_state", "external_lex_state"],
        "TSLexerMode" => &["lex_state", "external_lex_state", "reserved_word_set_id"],
        "TSCharacterRange" => &["start", "end"],
        _ => return None,
    })
}

const SCALAR_TYPES: [&str; 5] = ["TSSymbol", "TSStateId", "uint16_t", "uint32_t", "bool"];

struct Parser<'a> {
    src: &'a str,
    tz: Tokenizer,
    tok: Token,
    g: Grammar,
}

pub fn parse_grammar(src: &str) -> Result<Grammar, GenError> {
    let mut g = Grammar::default();
    for (name, v) in [
        ("ts_builtin_sym_end", 0),
        ("ts_builtin_sym_error", 0xFFFF),
        ("ts_builtin_sym_error_repeat", 0xFFFE),
    ] {
        g.consts.insert(name.to_string(), v);
    }
    let mut p = Parser {
        src,
        tz: Tokenizer::new(src),
        tok: Token {
            tok: Tok::Eof,
            pos: 0,
        },
        g,
    };
    p.next()?;
    while p.tok.tok != Tok::Eof {
        if p.accept("enum")? {
            p.enum_decl()?;
        } else if p.accept("static")? {
            p.static_decl()?;
        } else if p.accept("extern")? {
            // extern "C" {
            p.next()?;
            p.expect(&["{"])?;
        } else if p.accept("}")? {
            // closes the extern block
        } else if p.accept("TS_PUBLIC")? {
            p.language()?;
        } else if p.is("void") || p.is("unsigned") || p.is("bool") {
            // scanner prototypes; Go loops forever when one is cut off at the end of the input
            while !p.accept(";")? {
                if p.tok.tok == Tok::Eof {
                    return p.fail("unterminated scanner prototype");
                }
                p.next()?;
            }
        } else {
            let s = p.text(&p.tok);
            return p.fail(format!("unexpected {s:?} at top level"));
        }
    }
    p.g.defines = std::mem::take(&mut p.tz.defines);
    Ok(p.g)
}

/// One validated token of a lex condition, before the tree is built.
#[derive(Clone)]
enum Item {
    Lookahead,
    Eof,
    Lit(i64),
    Set(String, usize),
    Open,
    Close,
    Not,
    Cmp(CmpOp),
    And,
    Or,
}

/// A parsed expression: a condition, or an operand that only a comparison can use.
enum Expr {
    Cond(Cond),
    Operand(Operand),
}

fn as_cond(e: Expr) -> Result<Cond, String> {
    match e {
        Expr::Cond(c) => Ok(c),
        Expr::Operand(_) => Err("cannot evaluate operand as a condition".into()),
    }
}

fn as_operand(e: Expr) -> Result<Operand, String> {
    match e {
        Expr::Operand(o) => Ok(o),
        Expr::Cond(_) => Err("cannot evaluate condition as an operand".into()),
    }
}

/// Builds the tree from the items with Go's precedence: `||`, then `&&`, then the comparisons
/// (left associative), then `!` and parentheses.
struct CondParser {
    items: Vec<Item>,
    i: usize,
}

impl CondParser {
    fn or(&mut self) -> Result<Expr, String> {
        let mut left = self.and()?;
        while matches!(self.items.get(self.i), Some(Item::Or)) {
            self.i += 1;
            let right = self.and()?;
            left = Expr::Cond(Cond::Or(
                Box::new(as_cond(left)?),
                Box::new(as_cond(right)?),
            ));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut left = self.cmp()?;
        while matches!(self.items.get(self.i), Some(Item::And)) {
            self.i += 1;
            let right = self.cmp()?;
            left = Expr::Cond(Cond::And(
                Box::new(as_cond(left)?),
                Box::new(as_cond(right)?),
            ));
        }
        Ok(left)
    }

    fn cmp(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        while let Some(&Item::Cmp(op)) = self.items.get(self.i) {
            self.i += 1;
            let right = self.unary()?;
            left = Expr::Cond(Cond::Cmp(op, as_operand(left)?, as_operand(right)?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        let Some(item) = self.items.get(self.i).cloned() else {
            return Err("expected an expression".into());
        };
        self.i += 1;
        Ok(match item {
            Item::Lookahead => Expr::Operand(Operand::Lookahead),
            Item::Lit(v) => Expr::Operand(Operand::Lit(v)),
            Item::Eof => Expr::Cond(Cond::Eof),
            Item::Set(set, n) => Expr::Cond(Cond::SetContains { set, n }),
            Item::Not => Expr::Cond(Cond::Not(Box::new(as_cond(self.unary()?)?))),
            Item::Open => {
                let e = self.or()?;
                if !matches!(self.items.get(self.i), Some(Item::Close)) {
                    return Err("expected )".into());
                }
                self.i += 1;
                e
            }
            Item::Close | Item::Cmp(_) | Item::And | Item::Or => {
                return Err("expected an expression".into());
            }
        })
    }
}

impl Parser<'_> {
    fn fail<T>(&self, msg: impl Display) -> R<T> {
        Err(self.tz.error(self.tok.pos, msg))
    }

    fn next(&mut self) -> R<()> {
        self.tok = self.tz.next()?;
        Ok(())
    }

    /// The source text of a token, as Go's `token.s` (decoded bytes for a string).
    fn text(&self, t: &Token) -> String {
        match &t.tok {
            Tok::Eof => String::new(),
            Tok::Ident(s) | Tok::Char(_, s) => s.clone(),
            Tok::Num(_) => {
                let b = &self.src.as_bytes()[t.pos..];
                let n = b
                    .iter()
                    .position(|&c| !is_ident_byte(c, false))
                    .unwrap_or(b.len());
                self.src[t.pos..t.pos + n].to_string()
            }
            Tok::Str(b) => String::from_utf8_lossy(b).into_owned(),
            Tok::Punct(p) => p.to_string(),
        }
    }

    fn is(&self, s: &str) -> bool {
        match &self.tok.tok {
            Tok::Ident(x) => x == s,
            Tok::Punct(x) => *x == s,
            _ => false,
        }
    }

    fn accept(&mut self, s: &str) -> R<bool> {
        if self.is(s) {
            self.next()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn expect(&mut self, ss: &[&str]) -> R<()> {
        for s in ss {
            if !self.accept(s)? {
                let found = self.text(&self.tok);
                return self.fail(format!("expected {s:?}, found {found:?}"));
            }
        }
        Ok(())
    }

    fn ident(&mut self) -> R<String> {
        if let Tok::Ident(s) = &self.tok.tok {
            let s = s.clone();
            self.next()?;
            return Ok(s);
        }
        let found = self.text(&self.tok);
        self.fail(format!("expected identifier, found {found:?}"))
    }

    /// Consumes the separator after a list item and reports whether the list goes on.
    fn comma(&mut self, closer: &str) -> R<bool> {
        if self.accept(",")? {
            return Ok(!self.accept(closer)?);
        }
        self.expect(&[closer])?;
        Ok(false)
    }

    /// Evaluates the constant expressions `parser.c` uses in initialisers.
    fn expr(&mut self) -> R<i64> {
        let t = self.tok.clone();
        match &t.tok {
            Tok::Num(v) | Tok::Char(v, _) => {
                self.next()?;
                return Ok(*v);
            }
            Tok::Ident(s) => {
                self.next()?;
                match s.as_str() {
                    "true" => return Ok(1),
                    "false" => return Ok(0),
                    "ACTIONS" | "STATE" => {
                        // identity macros
                        self.expect(&["("])?;
                        let v = self.expr()?;
                        self.expect(&[")"])?;
                        return Ok(v);
                    }
                    "SMALL_STATE" => {
                        self.expect(&["("])?;
                        let v = self.expr()?;
                        self.expect(&[")"])?;
                        return Ok(v.wrapping_sub(self.define("LARGE_STATE_COUNT")?));
                    }
                    _ => {}
                }
                if let Some(v) = self.g.consts.get(s).or_else(|| self.tz.defines.get(s)) {
                    return Ok(*v);
                }
                return Err(self.tz.error(t.pos, format!("unknown identifier {s:?}")));
            }
            Tok::Punct("-") => {
                self.next()?;
                return Ok(self.expr()?.wrapping_neg());
            }
            Tok::Punct("(") => {
                self.next()?;
                if self.accept("TSStateId")? || self.accept("uint16_t")? {
                    self.expect(&[")"])?;
                    return Ok(self.expr()? & 0xFFFF);
                }
                let v = self.expr()?;
                self.expect(&[")"])?;
                return Ok(v);
            }
            _ => {}
        }
        let s = self.text(&t);
        self.fail(format!("unexpected {s:?} in expression"))
    }

    fn define(&self, name: &str) -> R<i64> {
        match self.tz.defines.get(name) {
            Some(v) => Ok(*v),
            None => self.fail(format!("missing #define {name}")),
        }
    }

    fn rng(&self, v: i64, lo: i64, hi: i64, what: &str) -> R<i64> {
        if !(lo..=hi).contains(&v) {
            return self.fail(format!("{what} {v} out of range"));
        }
        Ok(v)
    }

    fn enum_decl(&mut self) -> R<()> {
        let mut e = EnumDecl {
            name: self.ident()?,
            names: vec![],
            vals: vec![],
        };
        self.expect(&["{"])?;
        // C semantics: the counter starts at 0 in each enum.
        let mut next = 0i64;
        while !self.accept("}")? {
            let name = self.ident()?;
            if self.accept("=")? {
                next = self.expr()?;
            }
            if self.g.consts.contains_key(&name) {
                return self.fail(format!("duplicate constant {name}"));
            }
            self.g.consts.insert(name.clone(), next);
            e.names.push(name);
            e.vals.push(next);
            next = next.wrapping_add(1);
            if !self.comma("}")? {
                break;
            }
        }
        self.expect(&[";"])?;
        self.g.enums.push(e);
        Ok(())
    }

    fn static_decl(&mut self) -> R<()> {
        if self.accept("bool")? {
            return self.lex_fn();
        }
        self.accept("const")?; // ABI 14 character sets are not const
        let mut t = Table {
            name: String::new(),
            ctype: String::new(),
            dims: vec![],
            fields: None,
            vals: vec![],
            strs: vec![],
        };
        if self.accept("char")? {
            self.expect(&["*", "const"])?;
            t.ctype = "char*".to_string();
        } else {
            t.ctype = self.ident()?;
        }
        t.name = self.ident()?;
        while self.accept("[")? {
            let mut d = 0;
            if !self.is("]") {
                d = self.expr()?;
            }
            self.expect(&["]"])?;
            // Go would panic on a negative size later; refuse it here.
            let Ok(d) = usize::try_from(d) else {
                return self.fail(format!("{}: unsupported dimensions", t.name));
            };
            t.dims.push(d);
        }
        if t.dims.is_empty() || t.dims[1..].contains(&0) {
            return self.fail(format!("{}: unsupported dimensions", t.name));
        }
        self.expect(&["="])?;
        if self.g.tables.contains_key(&t.name) {
            return self.fail(format!("duplicate table {}", t.name));
        }
        if t.ctype == "char*" {
            self.strings(&mut t)?;
        } else if t.ctype == "TSParseActionEntry" {
            self.parse_actions()?;
        } else if struct_fields(&t.ctype).is_some() || SCALAR_TYPES.contains(&t.ctype.as_str()) {
            t.fields = struct_fields(&t.ctype);
            let dims = t.dims.clone();
            let n = (dims[0] as i64).wrapping_mul(t.width() as i64);
            if n < 0 {
                return self.fail(format!("{}: table size overflows", t.name));
            }
            t.vals = vec![0; n as usize];
            self.init(&mut t, 0, &dims)?;
        } else {
            return self.fail(format!("{}: unknown element type {}", t.name, t.ctype));
        }
        self.expect(&[";"])?;
        self.g.order.push(t.name.clone());
        self.g.tables.insert(t.name.clone(), t);
        Ok(())
    }

    /// Reads one initialiser into `t.vals[off..]`. Like C, a designator moves the cursor and the
    /// items after it continue from there.
    fn init(&mut self, t: &mut Table, off: i64, dims: &[usize]) -> R<()> {
        if dims.is_empty() && t.fields.is_none() {
            let v = self.expr()?;
            let Some(s) = usize::try_from(off).ok().and_then(|o| t.vals.get_mut(o)) else {
                return self.fail(format!("{}: index out of range", t.name));
            };
            *s = v;
            return Ok(());
        }
        self.expect(&["{"])?;
        if self.accept("}")? {
            return Ok(());
        }
        if dims.is_empty() {
            let fields = t.fields.unwrap_or(&[]);
            let mut fi = 0;
            loop {
                if self.accept(".")? {
                    let name = self.ident()?;
                    let Some(i) = fields.iter().position(|f| *f == name) else {
                        return self.fail(format!("{} has no field {name}", t.ctype));
                    };
                    fi = i;
                    self.expect(&["="])?;
                }
                if fi >= fields.len() {
                    return self.fail(format!("too many initialisers for {}", t.ctype));
                }
                let v = self.expr()?;
                let Some(s) = usize::try_from(off.wrapping_add(fi as i64))
                    .ok()
                    .and_then(|o| t.vals.get_mut(o))
                else {
                    return self.fail(format!("{}: index out of range", t.name));
                };
                *s = v;
                if !self.comma("}")? {
                    return Ok(());
                }
                fi += 1;
            }
        }
        let stride = dims[1..]
            .iter()
            .fold(t.fields.map_or(0, |f| f.len()).max(1), |s, &d| {
                s.wrapping_mul(d)
            }) as i64;
        let mut idx = 0i64;
        loop {
            if self.accept("[")? {
                idx = self.expr()?;
                self.expect(&["]", "="])?;
            }
            if idx < 0 || dims[0] != 0 && idx as usize >= dims[0] {
                return self.fail(format!("{}: index {idx} out of range", t.name));
            }
            let need = off.wrapping_add(idx.wrapping_add(1).wrapping_mul(stride));
            if (t.vals.len() as i64) < need {
                t.vals.resize(need as usize, 0);
            }
            self.init(t, off.wrapping_add(idx.wrapping_mul(stride)), &dims[1..])?;
            if !self.comma("}")? {
                return Ok(());
            }
            idx = idx.wrapping_add(1);
        }
    }

    fn strings(&mut self, t: &mut Table) -> R<()> {
        self.expect(&["{"])?;
        let mut idx = 0i64;
        while !self.accept("}")? {
            if self.accept("[")? {
                idx = self.expr()?;
                self.expect(&["]", "="])?;
            }
            if idx < 0 {
                // Go indexes a slice with it and panics.
                return self.fail(format!("{}: index {idx} out of range", t.name));
            }
            let idx_u = idx as usize;
            if t.strs.len() <= idx_u {
                t.strs.resize(idx_u + 1, String::new());
            }
            if !self.accept("NULL")? {
                let Tok::Str(b) = &self.tok.tok else {
                    return self.fail("expected string");
                };
                // The runtime reads these as C strings, which end at a NUL (Go's symbol "\0" is
                // the C name "").
                let cut = b.split(|&c| c == 0).next().unwrap_or(&[]);
                // gengrammar keeps names as `String`, so a name that is not UTF-8 is refused.
                let Ok(s) = String::from_utf8(cut.to_vec()) else {
                    return self.fail("string is not valid UTF-8");
                };
                t.strs[idx_u] = s;
                self.next()?;
            }
            if !self.comma("}")? {
                break;
            }
            idx += 1;
        }
        if t.dims[0] != 0 && t.strs.len() != t.dims[0] {
            return self.fail(format!(
                "{}: {} strings for dimension {}",
                t.name,
                t.strs.len(),
                t.dims[0]
            ));
        }
        Ok(())
    }

    /// Reads `ts_parse_actions` into 5-field slots: kind, child count or reusable, symbol or
    /// state, dynamic precedence or flags, production id.
    fn parse_actions(&mut self) -> R<()> {
        self.expect(&["{"])?;
        let mut idx = 0i64;
        while !self.accept("}")? {
            if self.accept("[")? {
                idx = self.expr()?;
                self.expect(&["]", "="])?;
            }
            if idx < 0 {
                return self.fail(format!("ts_parse_actions: index {idx} out of range"));
            }
            let mut a = [0i64; 5];
            if self.accept("{")? {
                self.expect(&[".", "entry", "=", "{", ".", "count", "="])?;
                let v = self.expr()?;
                a[0] = self.rng(v, 0, 0xFF, "entry.count")?;
                self.expect(&[",", ".", "reusable", "="])?;
                let v = self.expr()?;
                a[1] = self.rng(v, 0, 1, "entry.reusable")?;
                self.expect(&["}", "}"])?;
            } else {
                let name = self.ident()?;
                let mut args = vec![];
                self.expect(&["("])?;
                while !self.accept(")")? {
                    args.push(self.expr()?);
                    if !self.comma(")")? {
                        break;
                    }
                }
                let want = match name.as_str() {
                    "SHIFT" | "SHIFT_REPEAT" => 1,
                    "REDUCE" => 4,
                    _ => 0,
                };
                if args.len() != want {
                    return self.fail(format!("{name} with {} arguments", args.len()));
                }
                match name.as_str() {
                    "SHIFT" => a[2] = self.rng(args[0], 0, 0xFFFF, "shift state")?,
                    "SHIFT_REPEAT" => {
                        a[2] = self.rng(args[0], 0, 0xFFFF, "shift state")?;
                        a[3] = 2;
                    }
                    "SHIFT_EXTRA" => a[3] = 1,
                    "REDUCE" => {
                        a[0] = 1;
                        a[1] = self.rng(args[1], 0, 0xFF, "child_count")?;
                        a[2] = self.rng(args[0], 0, 0xFFFF, "symbol")?;
                        a[3] = self.rng(args[2], -32768, 32767, "dynamic_precedence")?;
                        a[4] = self.rng(args[3], 0, 0xFFFF, "production_id")?;
                    }
                    "ACCEPT_INPUT" => a[0] = 2,
                    "RECOVER" => a[0] = 3,
                    _ => return self.fail(format!("unknown parse action {name}")),
                }
            }
            let need = idx.wrapping_add(1).wrapping_mul(5);
            if (self.g.actions.len() as i64) < need {
                self.g.actions.resize(need as usize, 0);
            }
            let start = idx.wrapping_mul(5);
            let Some(dst) = usize::try_from(start)
                .ok()
                .and_then(|s| self.g.actions.get_mut(s..s.wrapping_add(5)))
            else {
                return self.fail("parse action index out of range");
            };
            dst.copy_from_slice(&a);
            if !self.comma("}")? {
                break;
            }
            idx += 1;
        }
        Ok(())
    }

    fn lex_fn(&mut self) -> R<()> {
        let mut f = LexFunc {
            cname: self.ident()?,
            cases: vec![],
        };
        self.expect(&[
            "(",
            "TSLexer",
            "*",
            "lexer",
            ",",
            "TSStateId",
            "state",
            ")",
            "{",
            "START_LEXER",
            "(",
            ")",
            ";",
            "eof",
            "=",
            "lexer",
            "->",
            "eof",
            "(",
            "lexer",
            ")",
            ";",
            "switch",
            "(",
            "state",
            ")",
            "{",
        ])?;
        let mut seen = HashSet::new();
        while self.accept("case")? {
            let v = self.expr()?;
            let state = self.rng(v, 0, 0xFFFF, "lex state")? as u16;
            self.expect(&[":"])?;
            if !seen.insert(state) {
                return self.fail(format!("duplicate case {state}"));
            }
            let mut stmts = vec![];
            while !self.is("case") && !self.is("default") {
                stmts.push(self.lex_stmt()?);
            }
            // Each C case must end in END_STATE, or it would fall into the next.
            for (i, s) in stmts.iter().enumerate() {
                if matches!(s, Stmt::End) != (i == stmts.len() - 1) {
                    return self.fail(format!("case {state}: END_STATE is not the last statement"));
                }
            }
            f.cases.push(LexCase { state, stmts });
        }
        self.expect(&["default", ":", "return", "false", ";", "}", "}"])?;
        self.g.lex_fns.push(f);
        Ok(())
    }

    fn lex_stmt(&mut self) -> R<Stmt> {
        if self.accept("END_STATE")? {
            self.expect(&["(", ")", ";"])?;
            return Ok(Stmt::End);
        }
        if self.accept("ACCEPT_TOKEN")? {
            self.expect(&["("])?;
            let pos = self.tok.pos;
            let sym = self.ident()?;
            if !self.g.consts.contains_key(&sym) {
                return Err(self.tz.error(pos, format!("unknown symbol {sym}")));
            }
            self.expect(&[")", ";"])?;
            return Ok(Stmt::Accept(sym));
        }
        if self.accept("ADVANCE_MAP")? {
            self.expect(&["("])?;
            let mut seen = HashSet::new();
            let mut pairs = vec![];
            while !self.accept(")")? {
                // C stores keys and states in a uint16_t map. A wider key would be cut there, so
                // refuse it.
                let v = self.expr()?;
                let k = self.rng(v, 0, 0xFFFF, "ADVANCE_MAP key")?;
                self.expect(&[","])?;
                let v = self.expr()?;
                let state = self.rng(v, 0, 0xFFFF, "ADVANCE_MAP state")?;
                if !seen.insert(k) {
                    return self.fail(format!("duplicate ADVANCE_MAP key {k}"));
                }
                pairs.push((k as u16, state as u16));
                if !self.comma(")")? {
                    break;
                }
            }
            self.expect(&[";"])?;
            return Ok(Stmt::Map(pairs));
        }
        if self.accept("if")? {
            self.expect(&["("])?;
            let cond = self.lex_cond()?;
            let skip = if self.accept("SKIP")? {
                true
            } else if self.accept("ADVANCE")? {
                false
            } else {
                let found = self.text(&self.tok);
                return self.fail(format!("expected ADVANCE or SKIP, found {found:?}"));
            };
            self.expect(&["("])?;
            let v = self.expr()?;
            let target = self.rng(v, 0, 0xFFFF, "lex state")? as u16;
            self.expect(&[")", ";"])?;
            return Ok(Stmt::If { cond, skip, target });
        }
        let found = self.text(&self.tok);
        self.fail(format!("unexpected {found:?} in lex function"))
    }

    /// Reads a C condition up to its closing parenthesis and builds its tree. Go copies it as Go
    /// source and re-parses it with `go/parser`; the operators used have the same meaning and
    /// precedence in both languages. What Go refuses at evaluation time is refused here.
    fn lex_cond(&mut self) -> R<Cond> {
        let mut items = vec![];
        let mut depth: i64 = 1;
        let close_pos = loop {
            let t = self.tok.clone();
            self.next()?;
            match &t.tok {
                Tok::Char(v, _) | Tok::Num(v) => items.push(Item::Lit(*v)),
                Tok::Ident(s) if s == "lookahead" => items.push(Item::Lookahead),
                Tok::Ident(s) if s == "eof" => items.push(Item::Eof),
                Tok::Ident(s) if s == "set_contains" => {
                    self.expect(&["("])?;
                    let name = self.ident()?;
                    let len = match self.g.tables.get(&name) {
                        Some(set) if set.ctype == "TSCharacterRange" => set.len(),
                        _ => {
                            return Err(self
                                .tz
                                .error(t.pos, format!("unknown character set {name}")));
                        }
                    };
                    self.expect(&[","])?;
                    let n = self.expr()?;
                    if usize::try_from(n) != Ok(len) {
                        return Err(self.tz.error(
                            t.pos,
                            format!("set_contains({name}, {n}): table has {len} ranges"),
                        ));
                    }
                    self.expect(&[",", "lookahead", ")"])?;
                    items.push(Item::Set(name, len));
                }
                Tok::Punct("(") => {
                    depth += 1;
                    items.push(Item::Open);
                }
                Tok::Punct(")") => {
                    depth -= 1;
                    if depth == 0 {
                        break t.pos;
                    }
                    items.push(Item::Close);
                }
                Tok::Punct("!") => items.push(Item::Not),
                Tok::Punct("==") => items.push(Item::Cmp(CmpOp::Eq)),
                Tok::Punct("!=") => items.push(Item::Cmp(CmpOp::Ne)),
                Tok::Punct("<") => items.push(Item::Cmp(CmpOp::Lt)),
                Tok::Punct(">") => items.push(Item::Cmp(CmpOp::Gt)),
                Tok::Punct("<=") => items.push(Item::Cmp(CmpOp::Le)),
                Tok::Punct(">=") => items.push(Item::Cmp(CmpOp::Ge)),
                Tok::Punct("&&") => items.push(Item::And),
                Tok::Punct("||") => items.push(Item::Or),
                _ => {
                    let s = self.text(&t);
                    return Err(self
                        .tz
                        .error(t.pos, format!("unexpected {s:?} in lex condition")));
                }
            }
        };
        let mut cp = CondParser { items, i: 0 };
        let built = cp.or().and_then(|e| {
            if cp.i < cp.items.len() {
                Err("unexpected tokens after the expression".to_string())
            } else {
                as_cond(e)
            }
        });
        built.map_err(|msg| self.tz.error(close_pos, format!("lex condition: {msg}")))
    }

    fn language(&mut self) -> R<()> {
        self.expect(&["const", "TSLanguage", "*"])?;
        let func = self.ident()?;
        let Some(name) = func.strip_prefix("tree_sitter_") else {
            return self.fail(format!("unexpected function {func}"));
        };
        if !self.g.name.is_empty() {
            return self.fail(format!("unexpected function {func}"));
        }
        self.g.name = name.to_string();
        self.expect(&[
            "(",
            "void",
            ")",
            "{",
            "static",
            "const",
            "TSLanguage",
            "language",
            "=",
        ])?;
        self.lang_fields("")?;
        self.expect(&[";", "return", "&", "language", ";", "}"])
    }

    fn lang_fields(&mut self, prefix: &str) -> R<()> {
        self.expect(&["{"])?;
        let mut i: i64 = 0;
        while !self.accept("}")? {
            let mut key = format!("{prefix}{i}");
            if self.accept(".")? {
                key = format!("{prefix}{}", self.ident()?);
                self.expect(&["="])?;
            }
            if self.is("{") {
                self.lang_fields(&format!("{key}."))?;
            } else {
                let mut toks = vec![];
                let mut depth: i64 = 0;
                while depth > 0 || !self.is(",") && !self.is("}") {
                    if self.tok.tok == Tok::Eof {
                        return self.fail("unterminated TSLanguage initialiser");
                    }
                    if self.is("(") || self.is("[") {
                        depth += 1;
                    } else if self.is(")") || self.is("]") {
                        depth -= 1;
                    }
                    toks.push(self.tok.clone());
                    self.next()?;
                }
                if self.g.lang.contains_key(&key) {
                    return self.fail(format!("duplicate TSLanguage field {key}"));
                }
                self.g.lang.insert(key, toks);
            }
            if !self.comma("}")? {
                break;
            }
            i += 1;
        }
        Ok(())
    }
}

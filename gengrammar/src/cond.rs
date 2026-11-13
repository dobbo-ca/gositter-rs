//! Lex condition AST. Go keeps a condition as Go source and re-parses it with `go/parser`; the
//! Rust port builds this tree in `parse.rs` from the same accepted grammar. Evaluation and range
//! lists come with the lexer emitter.

/// A comparison operator of a lex condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// One side of a comparison: `lookahead` or a literal (char literals are their value).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operand {
    Lookahead,
    Lit(i64),
}

/// A lex condition: `eof`, `!`, `&&`, `||`, a comparison, or `set_contains(set, n, lookahead)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cond {
    Eof,
    Not(Box<Cond>),
    And(Box<Cond>, Box<Cond>),
    Or(Box<Cond>, Box<Cond>),
    Cmp(CmpOp, Operand, Operand),
    SetContains { set: String, n: usize },
}

//! Shared by the parse and budget targets: the grammars of `tests/common` and the dump.

#[path = "../tests/common/mod.rs"]
pub mod common;

use std::sync::OnceLock;

use gositter::{Language, Parser};

/// The grammars gositter-grammars has today, in `common::DIRS` order.
pub fn languages() -> &'static [&'static Language] {
    static L: OnceLock<Vec<&'static Language>> = OnceLock::new();
    L.get_or_init(|| {
        common::DIRS
            .iter()
            .filter_map(|d| common::language(d))
            .collect()
    })
}

/// The treedump of a tree of `l` for `src` (fields as the regress runner).
pub fn dump_tree(tree: &gositter::Tree, l: &'static Language, src: &[u8]) -> Vec<u8> {
    let fields: &[&str] = l.tables.field_names.get(1..).unwrap_or_default();
    let mut out = Vec::new();
    gositter_treedump::dump(&mut out, tree.root_node(), fields, src);
    out
}

pub fn parser(l: &'static Language) -> Parser {
    let mut p = Parser::new();
    p.set_language(l).unwrap();
    p
}

/// Runs `f` on a big stack: treedump recurses once per tree level, and a seed from
/// testdata/regress can be deep.
pub fn on_big_stack(data: &[u8], f: fn(&[u8])) {
    let data = data.to_vec();
    common::run_on_big_stack(move || f(&data));
}

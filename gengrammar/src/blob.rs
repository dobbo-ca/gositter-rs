//! `tables.bin` writer and the checks of the `TSLanguage` initialiser (port of the tables and
//! language parts of `emit.go`).

use std::collections::HashMap;

use crate::ctok::{GenError, Tok};
use crate::parse::Grammar;

type R<T> = Result<T, GenError>;

fn fail<T>(msg: String) -> R<T> {
    Err(GenError(msg))
}

/// The numeric `TSLanguage` keys, as `(C key, Counts field, Rust type, largest value)`.
/// `version` is the old name of `abi_version`.
pub const COUNT_KEYS: [(&str, &str, &str, i64); 14] = [
    ("version", "abi_version", "u32", 0xFFFF_FFFF),
    ("abi_version", "abi_version", "u32", 0xFFFF_FFFF),
    ("symbol_count", "symbol_count", "u32", 0xFFFF_FFFF),
    ("alias_count", "alias_count", "u32", 0xFFFF_FFFF),
    ("token_count", "token_count", "u32", 0xFFFF_FFFF),
    (
        "external_token_count",
        "external_token_count",
        "u32",
        0xFFFF_FFFF,
    ),
    ("state_count", "state_count", "u32", 0xFFFF_FFFF),
    ("large_state_count", "large_state_count", "u32", 0xFFFF_FFFF),
    (
        "production_id_count",
        "production_id_count",
        "u32",
        0xFFFF_FFFF,
    ),
    ("field_count", "field_count", "u32", 0xFFFF_FFFF),
    (
        "max_alias_sequence_length",
        "max_alias_sequence_length",
        "u16",
        0xFFFF,
    ),
    (
        "keyword_capture_token",
        "keyword_capture_token",
        "u16",
        0xFFFF,
    ),
    (
        "max_reserved_word_set_size",
        "max_reserved_word_set_size",
        "u16",
        0xFFFF,
    ),
    ("supertype_count", "supertype_count", "u32", 0xFFFF_FFFF),
];

/// What `check_language` learned about the `TSLanguage` initialiser.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LangInfo {
    /// `(Counts field, value)` for every numeric key the initialiser sets.
    pub counts: Vec<(&'static str, i64)>,
    /// The tables the initialiser references, in `g.order`.
    pub used: Vec<String>,
    /// The `char*` tables behind `symbol_names` and `field_names` ("" if not set).
    pub symbol_names: String,
    pub field_names: String,
    /// `(gositter Language field, table)` for every table the initialiser references; the
    /// external scanner's are `States` and `SymbolMap`.
    pub fields: Vec<(String, String)>,
    /// `metadata.{major,minor,patch}_version`, 0 where not set.
    pub metadata: [i64; 3],
}

/// One table in `tables.bin`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    /// Byte offset, a multiple of 8.
    pub offset: usize,
    /// Elements, as the runtime view counts them.
    pub count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Blob {
    pub bytes: Vec<u8>,
    pub sections: Vec<Section>,
}

/// Go field name and value range of each struct element (`emit.go:36-47`), in C field order.
fn struct_spec(ctype: &str) -> Option<&'static [(&'static str, i64, i64)]> {
    Some(match ctype {
        "TSSymbolMetadata" => &[("Visible", 0, 1), ("Named", 0, 1), ("Supertype", 0, 1)],
        "TSFieldMapSlice" | "TSMapSlice" => &[("Index", 0, 0xFFFF), ("Length", 0, 0xFFFF)],
        "TSFieldMapEntry" => &[
            ("FieldID", 0, 0xFFFF),
            ("ChildIndex", 0, 0xFF),
            ("Inherited", 0, 1),
        ],
        "TSLexMode" => &[("LexState", 0, 0xFFFF), ("ExternalLexState", 0, 0xFFFF)],
        "TSLexerMode" => &[
            ("LexState", 0, 0xFFFF),
            ("ExternalLexState", 0, 0xFFFF),
            ("ReservedWordSetID", 0, 0xFFFF),
        ],
        "TSCharacterRange" => &[
            ("Start", -(1 << 31), (1 << 31) - 1),
            ("End", -(1 << 31), (1 << 31) - 1),
        ],
        "TSParseAction" => &[
            ("Type", 0, 0xFF),
            ("ChildCount", 0, 0xFF),
            ("Symbol", 0, 0xFFFF),
            ("DynamicPrecedence", -(1 << 15), (1 << 15) - 1),
            ("ProductionID", 0, 0xFFFF),
        ],
        _ => return None,
    })
}

fn scalar_range(ctype: &str) -> (i64, i64) {
    match ctype {
        "uint32_t" => (0, 0xFFFF_FFFF),
        "bool" => (0, 1),
        _ => (0, 0xFFFF),
    }
}

/// Every value fits its C type, and every parse action entry fits the table (`emit.go:116-214`).
fn check_values(g: &Grammar) -> R<()> {
    for en in &g.enums {
        for (n, v) in en.names.iter().zip(&en.vals) {
            if !(0..=0xFFFF).contains(v) {
                return fail(format!("{n}: value {v} does not fit u16"));
            }
        }
    }
    for name in &g.order {
        let t = &g.tables[name];
        match t.ctype.as_str() {
            "char*" => {}
            "TSParseActionEntry" => check_actions(name, &g.actions)?,
            ct => {
                if let Some(spec) = struct_spec(ct) {
                    check_elems(name, &t.vals, spec)?;
                } else {
                    let (lo, hi) = scalar_range(ct);
                    if let Some(v) = t.vals.iter().find(|v| !(lo..=hi).contains(*v)) {
                        return fail(format!("{name}: value {v} does not fit {ct}"));
                    }
                }
            }
        }
    }
    for f in &g.lex_fns {
        if !matches!(f.cname.as_str(), "ts_lex" | "ts_lex_keywords") {
            return fail(format!("unexpected lex function {}", f.cname));
        }
    }
    Ok(())
}

fn check_elems(name: &str, vals: &[i64], spec: &[(&str, i64, i64)]) -> R<()> {
    for el in vals.chunks(spec.len()) {
        for (v, (field, lo, hi)) in el.iter().zip(spec) {
            if !(*lo..=*hi).contains(v) {
                return fail(format!("{name}: value {v} does not fit {field}"));
            }
        }
    }
    Ok(())
}

/// A header slot says how many action slots follow it.
fn check_actions(name: &str, a: &[i64]) -> R<()> {
    let spec = struct_spec("TSParseAction").unwrap_or_default();
    let n = a.len() / 5;
    let mut i = 0;
    while i < n {
        let per = (a[i * 5] as usize).saturating_add(1);
        if per > n - i {
            return fail(format!("{name}: entry {i} runs past the end"));
        }
        check_elems(name, &a[i * 5..(i + per) * 5], spec)?;
        i += per;
    }
    Ok(())
}

struct Lang<'a> {
    g: &'a mut Grammar,
    num: Vec<(&'static str, i64)>,
    want: HashMap<&'static str, i64>,
    used: Vec<String>,
    info: LangInfo,
}

/// Reads a `TSLanguage` field that holds one constant.
fn eval(g: &mut Grammar, key: &str) -> R<Option<i64>> {
    let Some(toks) = g.lang.remove(key) else {
        return Ok(None);
    };
    if let [t] = toks.as_slice() {
        match &t.tok {
            Tok::Num(v) => return Ok(Some(*v)),
            Tok::Ident(s) => {
                if let Some(v) = g.consts.get(s).or_else(|| g.defines.get(s)) {
                    return Ok(Some(*v));
                }
            }
            _ => {}
        }
    }
    fail(format!("TSLanguage.{key}: cannot evaluate"))
}

/// Reads a `TSLanguage` field that points at a table or function and returns the name.
fn name_ref(g: &mut Grammar, key: &str) -> R<Option<String>> {
    let Some(toks) = g.lang.remove(key) else {
        return Ok(None);
    };
    let mut name: Option<String> = None;
    for t in &toks {
        if let Tok::Ident(s) = &t.tok
            && (s.starts_with("ts_") || s.starts_with("tree_sitter_"))
        {
            if name.is_some() {
                return fail(format!("TSLanguage.{key}: two names"));
            }
            name = Some(s.clone());
        }
    }
    match name {
        Some(n) => Ok(Some(n)),
        None => fail(format!("TSLanguage.{key}: no table name")),
    }
}

impl Lang<'_> {
    fn n(&self, field: &str) -> i64 {
        self.num.iter().find(|c| c.0 == field).map_or(0, |c| c.1)
    }

    /// A table field: the table must exist and have the element count the runtime indexes by.
    fn table(&mut self, key: &str, field: &str) -> R<Option<String>> {
        let Some(name) = name_ref(self.g, key)? else {
            return Ok(None);
        };
        let Some(t) = self.g.tables.get(&name) else {
            return fail(format!("TSLanguage.{key}: no table {name}"));
        };
        let n = match t.ctype.as_str() {
            "TSParseActionEntry" => self.g.actions.len() / 5,
            "char*" => t.strs.len(),
            _ => t.vals.len() / t.fields.map_or(1, |f| f.len()).max(1),
        };
        if let Some(&w) = self.want.get(field)
            && w != n as i64
        {
            return fail(format!("{name} has {n} entries, the counts say {w}"));
        }
        self.used.push(name.clone());
        self.info.fields.push((field.to_string(), name.clone()));
        match field {
            "SymbolNames" => self.info.symbol_names = name.clone(),
            "FieldNames" => self.info.field_names = name.clone(),
            _ => {}
        }
        Ok(Some(name))
    }
}

/// Checks the `TSLanguage` initialiser and every table value, as `emit.go:116-214` and
/// `emit.go:426-587`. Consumes the entries of `g.lang` it understands. Messages carry no grammar
/// prefix: the caller adds it.
pub fn check_language(g: &mut Grammar) -> Result<LangInfo, GenError> {
    check_values(g)?;
    let mut num: Vec<(&'static str, i64)> = vec![];
    for (key, field, ty, hi) in COUNT_KEYS {
        if let Some(v) = eval(g, key)? {
            if num.iter().any(|c| c.0 == field) {
                return fail(format!("TSLanguage.{key} set twice"));
            }
            if !(0..=hi).contains(&v) {
                return fail(format!("TSLanguage.{key}: value {v} does not fit {ty}"));
            }
            num.push((field, v));
        }
    }
    let mut l = Lang {
        g,
        num,
        want: HashMap::new(),
        used: vec![],
        info: LangInfo::default(),
    };
    let abi = l.n("abi_version");
    if abi != l.g.defines.get("LANGUAGE_VERSION").copied().unwrap_or(0) || abi == 0 {
        return fail(format!("ABI version {abi} does not match LANGUAGE_VERSION"));
    }

    let states = l.n("state_count");
    let large = l.n("large_state_count");
    let syms = l.n("symbol_count") + l.n("alias_count");
    let prod = l.n("production_id_count");
    l.want = HashMap::from([
        ("ParseTable", large * l.n("symbol_count")),
        ("SmallParseTableMap", states - large),
        ("SymbolNames", syms),
        ("SymbolMetadata", syms),
        ("PublicSymbolMap", syms),
        ("FieldNames", l.n("field_count") + 1),
        ("FieldMapSlices", prod),
        ("AliasSequences", prod * l.n("max_alias_sequence_length")),
        ("LexModes", states),
        ("PrimaryStateIDs", states),
        ("SupertypeSymbols", l.n("supertype_count")),
    ]);
    for (key, field) in [
        ("parse_table", "ParseTable"),
        ("small_parse_table", "SmallParseTable"),
        ("small_parse_table_map", "SmallParseTableMap"),
        ("parse_actions", "ParseActions"),
        ("symbol_names", "SymbolNames"),
        ("field_names", "FieldNames"),
        ("field_map_slices", "FieldMapSlices"),
        ("field_map_entries", "FieldMapEntries"),
        ("symbol_metadata", "SymbolMetadata"),
        ("public_symbol_map", "PublicSymbolMap"),
        ("alias_map", "AliasMap"),
        ("alias_sequences", "AliasSequences"),
        ("lex_modes", "LexModes"),
        ("primary_state_ids", "PrimaryStateIDs"),
        ("supertype_symbols", "SupertypeSymbols"),
        ("supertype_map_slices", "SupertypeMapSlices"),
        ("supertype_map_entries", "SupertypeMapEntries"),
    ] {
        l.table(key, field)?;
    }
    if let Some(name) = l.table("reserved_words", "ReservedWords")? {
        let w = l.g.tables[&name].width() as i64;
        let m = l.n("max_reserved_word_set_size");
        if w != m {
            return fail(format!(
                "ts_reserved_words rows are {w} wide, max_reserved_word_set_size is {m}"
            ));
        }
    }

    let f = name_ref(l.g, "lex_fn")?.unwrap_or_default();
    if f != "ts_lex" {
        return fail(format!("lex_fn is {f:?}"));
    }
    let f = name_ref(l.g, "keyword_lex_fn")?.unwrap_or_default();
    if !f.is_empty() && f != "ts_lex_keywords" {
        return fail(format!("keyword_lex_fn is {f:?}"));
    }
    let fns = if l.num.iter().any(|c| c.0 == "keyword_capture_token") {
        2
    } else {
        1
    };
    if l.g.lex_fns.len() != fns {
        return fail(format!(
            "{} lex functions, TSLanguage uses {fns}",
            l.g.lex_fns.len()
        ));
    }

    // `name` is the <name> of tree_sitter_<name>, also where C has NULL.
    if let Some(toks) = l.g.lang.remove("name") {
        let same = matches!(toks.as_slice(), [t] if matches!(&t.tok, Tok::Str(b) if b.as_slice() == l.g.name.as_bytes()));
        if !same {
            return fail(format!(
                "TSLanguage.name does not match tree_sitter_{}",
                l.g.name
            ));
        }
    }

    if l.g.lang.contains_key("metadata.major_version") {
        for (i, k) in ["major_version", "minor_version", "patch_version"]
            .into_iter()
            .enumerate()
        {
            let v = eval(l.g, &format!("metadata.{k}"))?.unwrap_or(0);
            if !(0..=0xFF).contains(&v) {
                return fail(format!("metadata.{k} out of range"));
            }
            l.info.metadata[i] = v;
        }
    }

    if l.g.lang.contains_key("external_scanner.0") {
        let ext = l.n("external_token_count");
        let name = l
            .table("external_scanner.0", "States")?
            .expect("key is present");
        let w = l.g.tables[&name].width() as i64;
        if w != ext {
            return fail(format!(
                "{name} rows are {w} wide, external_token_count is {ext}"
            ));
        }
        let name = l
            .table("external_scanner.1", "SymbolMap")?
            .expect("key is present");
        let n = l.g.tables[&name].vals.len() as i64;
        if n != ext {
            return fail(format!(
                "{name} has {n} entries, external_token_count is {ext}"
            ));
        }
        for (i, f) in ["create", "destroy", "scan", "serialize", "deserialize"]
            .into_iter()
            .enumerate()
        {
            let got = name_ref(l.g, &format!("external_scanner.{}", i + 2))?.unwrap_or_default();
            if got != format!("tree_sitter_{}_external_scanner_{f}", l.g.name) {
                return fail(format!("external_scanner.{f} is {got:?}"));
            }
        }
    } else if l.n("external_token_count") != 0 {
        return fail("external tokens without external_scanner".into());
    }

    if let Some(k) = l.g.lang.keys().next() {
        return fail(format!("TSLanguage.{k} is not handled"));
    }
    for name in &l.g.order {
        if !l.used.contains(name) && l.g.tables[name].ctype != "TSCharacterRange" {
            return fail(format!("table {name} is not referenced by TSLanguage"));
        }
    }
    let used = l.g.order.iter().filter(|n| l.used.contains(n)).cloned();
    l.info.used = used.collect();
    l.info.counts = l.num;
    Ok(l.info)
}

fn put(b: &mut Vec<u8>, v: i64, width: usize) {
    b.extend_from_slice(&v.to_le_bytes()[..width]);
}

/// Byte widths of the fields of one element. A `TSLexMode` has no `reserved_word_set_id`: it is
/// written as 0.
fn widths(ctype: &str) -> &'static [usize] {
    match ctype {
        "TSFieldMapSlice" | "TSMapSlice" => &[2, 2],
        "TSFieldMapEntry" => &[2, 1, 1],
        "TSLexMode" | "TSLexerMode" => &[2, 2, 2],
        "uint32_t" => &[4],
        "bool" => &[1],
        _ => &[2],
    }
}

/// The `tables.bin` of a grammar that `check_language` accepted: every referenced table that is
/// not a `char*` or `TSCharacterRange` table, in `g.order`, each padded to 8 bytes.
pub fn write_blob(g: &Grammar, info: &LangInfo) -> Blob {
    let mut b = Blob::default();
    for name in &g.order {
        let t = &g.tables[name];
        if !info.used.contains(name) || matches!(t.ctype.as_str(), "char*" | "TSCharacterRange") {
            continue;
        }
        let offset = b.bytes.len();
        let count = if t.ctype == "TSParseActionEntry" {
            let (slots, _) = g.actions.as_chunks::<5>();
            for s in slots {
                for (v, w) in s.iter().zip([1, 1, 2, 2, 2]) {
                    put(&mut b.bytes, *v, w);
                }
            }
            slots.len()
        } else {
            let nf = t.fields.map_or(1, |f| f.len());
            for el in t.vals.chunks(nf) {
                if t.ctype == "TSSymbolMetadata" {
                    b.bytes.push((el[0] | (el[1] << 1) | (el[2] << 2)) as u8);
                } else {
                    for (i, &w) in widths(&t.ctype).iter().enumerate() {
                        put(&mut b.bytes, el.get(i).copied().unwrap_or(0), w);
                    }
                }
            }
            t.vals.len() / nf
        };
        b.bytes.resize(b.bytes.len().next_multiple_of(8), 0);
        b.sections.push(Section {
            name: name.clone(),
            offset,
            count,
        });
    }
    b
}

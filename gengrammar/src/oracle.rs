//! The table oracle: our tables for a grammar against the tables gositter's own generated Go
//! code holds (`gengrammar/oracle/tables/main.go` dumps those).

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use crate::blob::{check_language, write_blob};
use crate::lock::{LockEntry, check_sum};
use crate::parse::parse_grammar;

/// Everything the oracle compares for one grammar.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OracleData {
    /// Gositter `Language` field name (`ParseActions`, `ExternalScanner.States`, ...) to the bytes
    /// of its elements. A field the grammar does not set is absent; an empty slice is present.
    pub fields: BTreeMap<String, Vec<u8>>,
    pub symbol_names: Vec<String>,
    pub field_names: Vec<String>,
    /// `Key` to value text: the numeric fields, `Name` and `Metadata.*Version`.
    pub counts: BTreeMap<String, String>,
}

/// Bytes per element of each table field, as `tables/main.go` writes them.
const ELEM_SIZE: [(&str, usize); 18] = [
    ("ParseTable", 2),
    ("SmallParseTable", 2),
    ("SmallParseTableMap", 4),
    ("ParseActions", 8),
    ("SymbolMetadata", 1),
    ("PublicSymbolMap", 2),
    ("AliasMap", 2),
    ("AliasSequences", 2),
    ("LexModes", 6),
    ("FieldMapSlices", 4),
    ("FieldMapEntries", 4),
    ("PrimaryStateIDs", 2),
    ("SupertypeSymbols", 2),
    ("SupertypeMapSlices", 4),
    ("SupertypeMapEntries", 2),
    ("ReservedWords", 2),
    ("ExternalScanner.States", 1),
    ("ExternalScanner.SymbolMap", 2),
];

/// Go key and `TSLanguage` field of each count, in the order `counts.txt` lists them.
const COUNTS: [(&str, &str); 13] = [
    ("ABIVersion", "abi_version"),
    ("SymbolCount", "symbol_count"),
    ("AliasCount", "alias_count"),
    ("TokenCount", "token_count"),
    ("ExternalTokenCount", "external_token_count"),
    ("StateCount", "state_count"),
    ("LargeStateCount", "large_state_count"),
    ("ProductionIDCount", "production_id_count"),
    ("FieldCount", "field_count"),
    ("MaxAliasSequenceLength", "max_alias_sequence_length"),
    ("KeywordCaptureToken", "keyword_capture_token"),
    ("MaxReservedWordSetSize", "max_reserved_word_set_size"),
    ("SupertypeCount", "supertype_count"),
];

fn elem_size(field: &str) -> usize {
    ELEM_SIZE.iter().find(|e| e.0 == field).map_or(1, |e| e.1)
}

/// The first element where two lists differ (or where the shorter one ends), with a message.
fn first_diff<T: PartialEq + std::fmt::Debug>(a: &[T], b: &[T]) -> Option<(usize, String)> {
    let i = a.iter().zip(b).position(|(x, y)| x != y);
    let i = i.or_else(|| (a.len() != b.len()).then_some(a.len().min(b.len())))?;
    Some((i, format!("ours {:?}, theirs {:?}", a.get(i), b.get(i))))
}

/// Compares our tables with gositter's. The error names the grammar, the field and the first
/// differing element, or the field present on one side only.
pub fn compare(grammar: &str, ours: &OracleData, theirs: &OracleData) -> Result<(), String> {
    for (k, a) in &ours.fields {
        let Some(b) = theirs.fields.get(k) else {
            return Err(format!("{grammar}: {k}: only in ours"));
        };
        if let Some((off, what)) = first_diff(a, b) {
            return Err(format!(
                "{grammar}: {k}: element {} differs (byte {off}): {what}; ours {} bytes, theirs {}",
                off / elem_size(k),
                a.len(),
                b.len()
            ));
        }
    }
    if let Some(k) = theirs.fields.keys().find(|k| !ours.fields.contains_key(*k)) {
        return Err(format!("{grammar}: {k}: only in theirs"));
    }
    for (k, a, b) in [
        ("SymbolNames", &ours.symbol_names, &theirs.symbol_names),
        ("FieldNames", &ours.field_names, &theirs.field_names),
    ] {
        if let Some((i, what)) = first_diff(a, b) {
            return Err(format!("{grammar}: {k}: element {i} differs: {what}"));
        }
    }
    for k in ours.counts.keys().chain(theirs.counts.keys()) {
        let (a, b) = (ours.counts.get(k), theirs.counts.get(k));
        if a != b {
            return Err(format!("{grammar}: {k}: ours {a:?}, theirs {b:?}"));
        }
    }
    Ok(())
}

fn bad(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

/// `u32` LE length, then that many bytes, per name.
fn read_names(b: &[u8]) -> io::Result<Vec<String>> {
    let (mut out, mut rest) = (vec![], b);
    while !rest.is_empty() {
        let (head, tail) = rest
            .split_at_checked(4)
            .ok_or_else(|| bad("name length cut".into()))?;
        let n = u32::from_le_bytes(head.try_into().unwrap()) as usize;
        let (s, tail) = tail
            .split_at_checked(n)
            .ok_or_else(|| bad("name cut".into()))?;
        out.push(String::from_utf8(s.to_vec()).map_err(|e| bad(e.to_string()))?);
        rest = tail;
    }
    Ok(out)
}

/// Reads what `tables/main.go` wrote for one grammar: `<Field>.bin` per set field, `SymbolNames.bin`,
/// `FieldNames.bin` and `counts.txt`.
pub fn read_oracle_dir(dir: &Path) -> io::Result<OracleData> {
    let mut d = OracleData::default();
    for e in std::fs::read_dir(dir)? {
        let path = e?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if let Some(field) = name.strip_suffix(".bin") {
            let bytes = std::fs::read(&path)?;
            match field {
                "SymbolNames" => d.symbol_names = read_names(&bytes)?,
                "FieldNames" => d.field_names = read_names(&bytes)?,
                _ => {
                    d.fields.insert(field.to_string(), bytes);
                }
            }
        } else if name == "counts.txt" {
            for line in std::fs::read_to_string(&path)?.lines() {
                let (k, v) = line
                    .split_once(' ')
                    .ok_or_else(|| bad(format!("counts.txt: bad line {line:?}")))?;
                d.counts.insert(k.into(), v.into());
            }
        }
    }
    Ok(d)
}

/// Our side: parses `parser.c`, checks the language and builds the blob in memory, as `generate`
/// in `main.rs` does, then maps each blob section to the gositter `Language` field it feeds.
pub fn oracle_from_source(src: &str) -> Result<OracleData, String> {
    let mut g = parse_grammar(src).map_err(|e| e.0)?;
    let info = check_language(&mut g).map_err(|e| e.0)?;
    let blob = write_blob(&g, &info);
    let mut d = OracleData::default();
    for (field, table) in &info.fields {
        let Some(sec) = blob.sections.iter().find(|s| &s.name == table) else {
            continue; // a char* table: the names below
        };
        let field = match field.as_str() {
            f @ ("States" | "SymbolMap") => format!("ExternalScanner.{f}"),
            f => f.to_string(),
        };
        let end = sec.offset + sec.count * elem_size(&field);
        let bytes = blob.bytes.get(sec.offset..end);
        let bytes = bytes.ok_or_else(|| format!("{table}: section runs past the blob"))?;
        d.fields.insert(field, bytes.to_vec());
    }
    let names = |t: &str| g.tables.get(t).map_or(vec![], |t| t.strs.clone());
    d.symbol_names = names(&info.symbol_names);
    d.field_names = names(&info.field_names);
    for (key, field) in COUNTS {
        let v = info.counts.iter().find(|c| c.0 == field).map_or(0, |c| c.1);
        d.counts.insert(key.into(), v.to_string());
    }
    d.counts.insert("Name".into(), g.name.clone());
    for (k, v) in ["Major", "Minor", "Patch"].iter().zip(info.metadata) {
        d.counts
            .insert(format!("Metadata.{k}Version"), v.to_string());
    }
    Ok(d)
}

/// Checks one grammar of the lock against `<oracle>/<pkg>/`. `src` is the `--src` directory.
/// Every error starts with the package name.
pub fn check_grammar(l: &LockEntry, src: &Path, oracle: &Path) -> Result<(), String> {
    let pre = |e: String| format!("{}: {e}", l.pkg);
    let path = l.parser_c(src);
    let text = std::fs::read(&path).map_err(|e| pre(format!("{}: {e}", path.display())))?;
    check_sum(&text, &l.sum, false).map_err(pre)?;
    let text = std::str::from_utf8(&text).map_err(|e| pre(format!("{}: {e}", path.display())))?;
    let ours = oracle_from_source(text).map_err(pre)?;
    let dir = oracle.join(&l.pkg);
    let theirs = read_oracle_dir(&dir).map_err(|e| pre(format!("{}: {e}", dir.display())))?;
    compare(&l.pkg, &ours, &theirs)
}

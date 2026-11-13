//! The table oracle: our tables for a grammar against the tables gositter's own generated Go
//! code holds (`gengrammar/oracle/tables/main.go` dumps those).

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use crate::lock::LockEntry;

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

#[allow(dead_code)] // tests-first stub
pub fn compare(grammar: &str, ours: &OracleData, theirs: &OracleData) -> Result<(), String> {
    let _ = (grammar, ours, theirs);
    todo!()
}

#[allow(dead_code)] // tests-first stub
pub fn read_oracle_dir(dir: &Path) -> io::Result<OracleData> {
    let _ = dir;
    todo!()
}

#[allow(dead_code)] // tests-first stub
pub fn oracle_from_source(src: &str) -> Result<OracleData, String> {
    let _ = src;
    todo!()
}

#[allow(dead_code)] // tests-first stub
pub fn check_grammar(l: &LockEntry, src: &Path, oracle: &Path) -> Result<(), String> {
    let _ = (l, src, oracle);
    todo!()
}

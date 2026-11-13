//! `tables.bin` writer and the checks of the `TSLanguage` initialiser (port of the tables and
//! language parts of `emit.go`).

use crate::ctok::GenError;
use crate::parse::Grammar;

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

/// Checks the `TSLanguage` initialiser and every table value, as `emit.go:116-214` and
/// `emit.go:426-587`. Consumes the entries of `g.lang` it understands.
pub fn check_language(g: &mut Grammar) -> Result<LangInfo, GenError> {
    let _ = g;
    todo!()
}

/// The `tables.bin` of a grammar that `check_language` accepted.
pub fn write_blob(g: &Grammar, info: &LangInfo) -> Blob {
    let _ = (g, info);
    todo!()
}

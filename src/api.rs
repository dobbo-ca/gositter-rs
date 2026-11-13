//! The frozen contract between the runtime, gengrammar and the hand-written scanners
//! (gositter v0.2.0 `api.go` and `CONTRACT.md`, adapted to Rust). A change needs a dated
//! entry in `docs/CONTRACT_CHANGES.md`.

/// TSSymbol.
pub type Symbol = u16;
/// TSStateId.
pub type StateId = u16;
/// TSFieldId.
pub type FieldId = u16;

/// ts_builtin_sym_end
pub const SYMBOL_END: Symbol = 0;
/// ts_builtin_sym_error
pub const SYMBOL_ERROR: Symbol = 0xFFFF;
/// ts_builtin_sym_error_repeat
pub const SYMBOL_ERROR_REPEAT: Symbol = 0xFFFE;

/// TREE_SITTER_SERIALIZATION_BUFFER_SIZE
pub const SERIALIZATION_BUFFER_SIZE: usize = 1024;

/// The newest grammar ABI version the runtime accepts (TREE_SITTER_LANGUAGE_VERSION).
pub const TREE_SITTER_LANGUAGE_VERSION: u32 = 15;
/// TREE_SITTER_MIN_COMPATIBLE_LANGUAGE_VERSION
pub const MIN_COMPATIBLE_LANGUAGE_VERSION: u32 = 13;
/// LANGUAGE_VERSION_WITH_RESERVED_WORDS
pub const LANGUAGE_VERSION_WITH_RESERVED_WORDS: u32 = 15;
/// LANGUAGE_VERSION_WITH_PRIMARY_STATES
pub const LANGUAGE_VERSION_WITH_PRIMARY_STATES: u32 = 14;

/// TSPoint. Column counts bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub row: u32,
    pub column: u32,
}

/// TSFieldMapEntry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FieldMapEntry {
    pub field_id: FieldId,
    pub child_index: u8,
    pub inherited: bool,
}

/// TSMapSlice. Indexes the field and supertype maps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapSlice {
    pub index: u16,
    pub length: u16,
}

/// TSSymbolMetadata.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SymbolMetadata {
    pub visible: bool,
    pub named: bool,
    pub supertype: bool,
}

/// TSLexerMode. ABI 14 grammars (TSLexMode) leave `reserved_word_set_id` zero.
/// `lex_state` 0xFFFF is the C `(uint16_t)-1` marker and must be kept.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LexMode {
    pub lex_state: u16,
    pub external_lex_state: u16,
    pub reserved_word_set_id: u16,
}

/// TSCharacterRange, inclusive on both ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CharacterRange {
    pub start: i32,
    pub end: i32,
}

/// TSLanguageMetadata.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LanguageMetadata {
    pub major_version: u8,
    pub minor_version: u8,
    pub patch_version: u8,
}

/// One 8-byte slot of the parse actions. It stands for both C unions, TSParseActionEntry
/// and TSParseAction (gositter CONTRACT.md section 4). Which variant a slot holds follows
/// from its position: a header slot is followed by `count()` action slots.
///
/// | slot    | action_type | child_count | symbol | dynamic_precedence    | production_id |
/// |---------|-------------|-------------|--------|-----------------------|---------------|
/// | header  | count       | reusable    | 0      | 0                     | 0             |
/// | shift   | 0           | 0           | state  | bit 0 extra, 1 repeat | 0             |
/// | reduce  | 1           | child_count | symbol | dynamic_precedence    | production_id |
/// | accept  | 2           | 0           | 0      | 0                     | 0             |
/// | recover | 3           | 0           | 0      | 0                     | 0             |
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ParseAction {
    pub action_type: u8,
    pub child_count: u8,
    pub symbol: Symbol,
    pub dynamic_precedence: i16,
    pub production_id: u16,
}

pub const PARSE_ACTION_TYPE_SHIFT: u8 = 0;
pub const PARSE_ACTION_TYPE_REDUCE: u8 = 1;
pub const PARSE_ACTION_TYPE_ACCEPT: u8 = 2;
pub const PARSE_ACTION_TYPE_RECOVER: u8 = 3;

const SHIFT_EXTRA: i16 = 1;
const SHIFT_REPETITION: i16 = 2;

impl ParseAction {
    /// entry.count of a header slot.
    pub const fn count(self) -> u8 {
        self.action_type
    }

    /// entry.reusable of a header slot.
    pub const fn reusable(self) -> bool {
        self.child_count != 0
    }

    /// shift.state
    pub const fn state(self) -> StateId {
        self.symbol
    }

    /// shift.extra
    pub const fn extra(self) -> bool {
        self.dynamic_precedence & SHIFT_EXTRA != 0
    }

    /// shift.repetition
    pub const fn repetition(self) -> bool {
        self.dynamic_precedence & SHIFT_REPETITION != 0
    }
}

/// A typed, little-endian view over static bytes (`include_bytes!` output needs no
/// alignment). `get` decodes with `from_le_bytes`.
macro_rules! view {
    ($(#[$doc:meta])* $name:ident, $elem:ty, $size:expr, |$b:ident| $decode:expr) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug)]
        pub struct $name(&'static [u8]);

        impl $name {
            /// The element size in bytes.
            pub const SIZE: usize = $size;
            /// A view of no elements.
            pub const EMPTY: Self = $name(&[]);

            /// The whole of `bytes`. Panics if its length is not a multiple of the
            /// element size.
            pub const fn new(bytes: &'static [u8]) -> Self {
                assert!(
                    bytes.len().is_multiple_of(Self::SIZE),
                    "view length is not a multiple of the element size"
                );
                $name(bytes)
            }

            /// `count` elements at byte `offset` of `blob`.
            pub const fn at(blob: &'static [u8], offset: usize, count: usize) -> Self {
                let (_, rest) = blob.split_at(offset);
                let (bytes, _) = rest.split_at(count * Self::SIZE);
                $name(bytes)
            }

            /// The number of elements.
            pub const fn len(self) -> usize {
                self.0.len() / Self::SIZE
            }

            pub const fn is_empty(self) -> bool {
                self.0.is_empty()
            }

            /// Element `i`. Panics if `i >= len()`.
            #[inline]
            pub fn get(self, i: usize) -> $elem {
                let $b = &self.0[i * Self::SIZE..i * Self::SIZE + Self::SIZE];
                $decode
            }
        }
    };
}

view!(
    /// u16 elements.
    U16s, u16, 2, |b| u16::from_le_bytes([b[0], b[1]])
);
view!(
    /// u32 elements.
    U32s, u32, 4, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])
);
view!(
    /// bool elements, one byte each, 0 or 1.
    Bools, bool, 1, |b| match b[0] {
        0 => false,
        1 => true,
        v => panic!("gositter: bool table byte {v}"),
    }
);
view!(
    /// ParseAction elements: action_type, child_count, symbol, dynamic_precedence,
    /// production_id.
    ParseActions, ParseAction, 8, |b| ParseAction {
        action_type: b[0],
        child_count: b[1],
        symbol: u16::from_le_bytes([b[2], b[3]]),
        dynamic_precedence: i16::from_le_bytes([b[4], b[5]]),
        production_id: u16::from_le_bytes([b[6], b[7]]),
    }
);
view!(
    /// LexMode elements: lex_state, external_lex_state, reserved_word_set_id.
    LexModes, LexMode, 6, |b| LexMode {
        lex_state: u16::from_le_bytes([b[0], b[1]]),
        external_lex_state: u16::from_le_bytes([b[2], b[3]]),
        reserved_word_set_id: u16::from_le_bytes([b[4], b[5]]),
    }
);
view!(
    /// SymbolMetadata elements: bit 0 visible, bit 1 named, bit 2 supertype.
    SymbolMetadatas, SymbolMetadata, 1, |b| SymbolMetadata {
        visible: b[0] & 1 != 0,
        named: b[0] & 2 != 0,
        supertype: b[0] & 4 != 0,
    }
);
view!(
    /// MapSlice elements: index, length.
    MapSlices, MapSlice, 4, |b| MapSlice {
        index: u16::from_le_bytes([b[0], b[1]]),
        length: u16::from_le_bytes([b[2], b[3]]),
    }
);
view!(
    /// FieldMapEntry elements: field_id (u16), child_index (u8), inherited (u8).
    FieldMapEntries, FieldMapEntry, 4, |b| FieldMapEntry {
        field_id: u16::from_le_bytes([b[0], b[1]]),
        child_index: b[2],
        inherited: b[3] != 0,
    }
);

/// The little-endian bytes of a u16 table, for hand-written tables (tests, fixtures).
/// Panics when `M != 2 * N`.
pub const fn le_bytes<const N: usize, const M: usize>(v: &[u16; N]) -> [u8; M] {
    assert!(M == 2 * N, "le_bytes: M must be 2 * N");
    let mut out = [0u8; M];
    let mut i = 0;
    while i < N {
        let b = v[i].to_le_bytes();
        out[2 * i] = b[0];
        out[2 * i + 1] = b[1];
        i += 1;
    }
    out
}

/// A generated ts_lex / ts_lex_keywords. gengrammar emits one non-generic wrapper per lex
/// function, because a generic function does not coerce to this higher-ranked pointer.
pub type LexFn = fn(&mut Lexer<'_>, StateId) -> bool;

/// One instance of a grammar's external scanner (gositter CONTRACT.md section 7).
pub trait ExternalScanner: Send {
    /// scan. `valid_symbols` has external_token_count entries. On success set
    /// `lexer.result_symbol` to the external token index.
    fn scan(&mut self, lexer: &mut Lexer<'_>, valid_symbols: &[bool]) -> bool;
    /// serialize. Returns the bytes written, which must be byte-identical to C.
    fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize;
    /// deserialize. An empty `buf` is the C `(NULL, 0)` reset. `buf` is only valid
    /// during the call.
    fn deserialize(&mut self, buf: &[u8]);
    /// Go's optional `Reset()`: put the scanner in exactly the state the constructor
    /// returns and return true. False (the default) makes the parser call the
    /// constructor instead.
    fn reset(&mut self) -> bool {
        false
    }
}

/// TSLanguage.external_scanner.
#[derive(Clone, Copy)]
pub struct ExternalScannerInfo {
    /// `[external state][external_token_count]`, row 0 unused.
    pub states: Bools,
    /// External token index to symbol.
    pub symbol_map: U16s,
    /// `None` if the grammar has no scanner.
    pub create: Option<fn() -> Box<dyn ExternalScanner>>,
}

/// TSLanguage (ABI 15) field for field, in C order. Two-dimensional C arrays are
/// flattened row-major. The gositter CONTRACT.md section 3 generator rules apply: gaps
/// are kept, `(uint16_t)-1` is 0xFFFF, `name` is always set, a missing field is zero,
/// `EMPTY` or `None`. Names are valid UTF-8.
#[derive(Clone, Copy)]
pub struct LanguageTables {
    pub abi_version: u32,
    pub symbol_count: u32,
    pub alias_count: u32,
    pub token_count: u32,
    pub external_token_count: u32,
    pub state_count: u32,
    pub large_state_count: u32,
    pub production_id_count: u32,
    pub field_count: u32,
    pub max_alias_sequence_length: u16,
    /// `[large_state_count][symbol_count]`
    pub parse_table: U16s,
    pub small_parse_table: U16s,
    /// Indexed by `state - large_state_count`.
    pub small_parse_table_map: U32s,
    pub parse_actions: ParseActions,
    /// `symbol_count + alias_count` entries.
    pub symbol_names: &'static [&'static str],
    /// `field_count + 1` entries, `[0] == ""`.
    pub field_names: &'static [&'static str],
    pub field_map_slices: MapSlices,
    pub field_map_entries: FieldMapEntries,
    pub symbol_metadata: SymbolMetadatas,
    pub public_symbol_map: U16s,
    /// Zero-terminated, as in C.
    pub alias_map: U16s,
    /// `[production_id_count][max_alias_sequence_length]`
    pub alias_sequences: U16s,
    /// `state_count` entries.
    pub lex_modes: LexModes,
    pub lex_fn: LexFn,
    /// `None` if the grammar has no word token.
    pub keyword_lex_fn: Option<LexFn>,
    pub keyword_capture_token: Symbol,
    pub external_scanner: ExternalScannerInfo,
    /// `state_count` entries.
    pub primary_state_ids: U16s,
    pub name: &'static str,
    /// `[set][max_reserved_word_set_size]`
    pub reserved_words: U16s,
    pub max_reserved_word_set_size: u16,
    pub supertype_count: u32,
    pub supertype_symbols: U16s,
    pub supertype_map_slices: MapSlices,
    pub supertype_map_entries: U16s,
    pub metadata: LanguageMetadata,
}

/// The tables plus the runtime's derived state (comb table, field-id cache, KindMask
/// bits). It is `Sync`, so a grammar holds it in a `static` built with `Language::new`
/// and returns `&'static Language`. `Clone` copies the tables and starts fresh derived
/// state.
pub struct Language {
    pub tables: LanguageTables,
    pub(crate) derived: crate::language::Derived,
}

impl Language {
    pub const fn new(tables: LanguageTables) -> Language {
        Language {
            tables,
            derived: crate::language::Derived::new(),
        }
    }
}

// The other Language methods are in language.rs (language.c).

/// The C Lexer with its embedded TSLexer, valid for one parse (`'s` borrows the source).
/// Generated lexers and scanners use only the two public fields and the methods below.
pub struct Lexer<'s> {
    /// Current code point: 0 at end of input (and for a NUL byte), -1 for invalid UTF-8.
    pub lookahead: i32,
    /// Set by lex functions (a symbol) and scanners (an external token index).
    pub result_symbol: Symbol,
    /// The whole input (TSInput with ts_string_input_read).
    pub(crate) src: &'s [u8],
    pub(crate) state: crate::lexer::LexerState,
}

// The Lexer methods are in lexer.rs (lexer.c).

/// set_contains: the C binary search, line for line.
pub fn set_contains(ranges: &[CharacterRange], lookahead: i32) -> bool {
    let mut index = 0usize;
    let mut size = ranges.len();
    while size > 1 {
        let half_size = size / 2;
        let mid_index = index + half_size;
        let range = &ranges[mid_index];
        if (range.start..=range.end).contains(&lookahead) {
            return true;
        } else if lookahead > range.end {
            index = mid_index;
        }
        size -= half_size;
    }
    let range = &ranges[index];
    (range.start..=range.end).contains(&lookahead)
}

/// set_contains with the answers for code points below 128 precomputed by gengrammar:
/// `ascii[c] == set_contains(ranges, c)`.
#[inline]
pub fn set_contains_ascii(ascii: &[bool; 128], ranges: &[CharacterRange], lookahead: i32) -> bool {
    if (lookahead as u32) < 128 {
        ascii[lookahead as usize]
    } else {
        set_contains(ranges, lookahead)
    }
}

/// TSLogType.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogType {
    Parse = 0,
    Lex = 1,
}

/// Receives the messages the C runtime passes to TSLogger.log. Messages are bytes: C cuts
/// them at 1,023 bytes, which can split a UTF-8 character.
pub type Logger = Box<dyn FnMut(LogType, &[u8]) + Send>;

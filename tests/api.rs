//! The frozen interface (src/api.rs): plan Task 1.

use gositter::*;

#[test]
fn parse_action_readers() {
    let header = ParseAction {
        action_type: 3,
        child_count: 1,
        ..Default::default()
    };
    assert_eq!((header.count(), header.reusable()), (3, true));
    let shift = ParseAction {
        symbol: 28,
        dynamic_precedence: 2,
        ..Default::default()
    };
    assert_eq!(
        (shift.state(), shift.extra(), shift.repetition()),
        (28, false, true)
    );
    let extra = ParseAction {
        dynamic_precedence: 1,
        ..Default::default()
    };
    assert_eq!((extra.extra(), extra.repetition()), (true, false));
}

#[test]
fn views_are_little_endian() {
    static B16: [u8; 6] = le_bytes::<3, 6>(&[1, 0x0203, 0xFFFF]);
    assert_eq!(B16, [1, 0, 3, 2, 0xFF, 0xFF]);
    let t = U16s::new(&B16);
    assert_eq!((t.len(), t.get(1), t.get(2)), (3, 0x0203, 0xFFFF));
    assert_eq!(U16s::at(&B16, 2, 2).get(0), 0x0203);
    assert_eq!(U16s::at(&B16, 2, 2).len(), 2);
    assert!(U16s::EMPTY.is_empty());
    static B32: [u8; 4] = [0x78, 0x56, 0x34, 0x12];
    assert_eq!(U32s::new(&B32).get(0), 0x1234_5678);
    static BA: [u8; 8] = [1, 2, 2, 1, 0xFE, 0xFF, 7, 0];
    assert_eq!(
        ParseActions::new(&BA).get(0),
        ParseAction {
            action_type: 1,
            child_count: 2,
            symbol: 0x0102,
            dynamic_precedence: -2,
            production_id: 7
        }
    );
    static LM: [u8; 6] = [0xFF, 0xFF, 1, 0, 2, 0];
    assert_eq!(
        LexModes::new(&LM).get(0),
        LexMode {
            lex_state: 0xFFFF,
            external_lex_state: 1,
            reserved_word_set_id: 2
        }
    );
    assert_eq!(
        SymbolMetadatas::new(&[0b101]).get(0),
        SymbolMetadata {
            visible: true,
            named: false,
            supertype: true
        }
    );
    assert_eq!(
        FieldMapEntries::new(&[2, 0, 1, 1]).get(0),
        FieldMapEntry {
            field_id: 2,
            child_index: 1,
            inherited: true
        }
    );
    assert_eq!(
        MapSlices::new(&[3, 0, 2, 0]).get(0),
        MapSlice {
            index: 3,
            length: 2
        }
    );
    let bools = Bools::new(&[0, 1]);
    assert!(!bools.get(0) && bools.get(1));
}

#[test]
#[should_panic(expected = "gositter: bool table byte 2")]
fn bools_reject_other_bytes() {
    Bools::new(&[2]).get(0);
}

#[test]
#[should_panic]
fn view_get_out_of_range_panics() {
    U16s::new(&[1, 0]).get(1);
}

#[test]
fn set_contains_matches_c_binary_search() {
    let r = [
        CharacterRange {
            start: 'a' as i32,
            end: 'c' as i32,
        },
        CharacterRange {
            start: 'x' as i32,
            end: 'z' as i32,
        },
    ];
    for c in "abcxyz".chars() {
        assert!(set_contains(&r, c as i32), "{c}");
    }
    for c in [b'`' as i32, b'd' as i32, b'w' as i32, b'{' as i32, -1] {
        assert!(!set_contains(&r, c), "{c}");
    }
    let one = [CharacterRange { start: 5, end: 5 }];
    assert!(set_contains(&one, 5) && !set_contains(&one, 6));
}

#[test]
fn set_contains_ascii_uses_table_below_128() {
    let mut ascii = [false; 128];
    ascii[b'q' as usize] = true;
    let r = [CharacterRange {
        start: 0x100,
        end: 0x200,
    }];
    assert!(set_contains_ascii(&ascii, &r, b'q' as i32));
    assert!(!set_contains_ascii(&ascii, &r, b'r' as i32));
    assert!(set_contains_ascii(&ascii, &r, 0x150));
    assert!(!set_contains_ascii(&ascii, &r, -1));
}

#[test]
fn auto_traits() {
    fn sync<T: Sync>() {}
    fn send<T: Send>() {}
    sync::<Language>();
    send::<Parser>();
    send::<Box<dyn ExternalScanner>>();
}

#[test]
fn language_error_text() {
    assert_eq!(
        LanguageError { abi_version: 12 }.to_string(),
        "gositter: incompatible language ABI version 12"
    );
}

#[test]
fn constants_match_gositter() {
    // gositter api.go:20-31.
    assert_eq!(
        (SYMBOL_END, SYMBOL_ERROR, SYMBOL_ERROR_REPEAT),
        (0, 0xFFFF, 0xFFFE)
    );
    assert_eq!(SERIALIZATION_BUFFER_SIZE, 1024);
    assert_eq!(
        (
            TREE_SITTER_LANGUAGE_VERSION,
            MIN_COMPATIBLE_LANGUAGE_VERSION,
            LANGUAGE_VERSION_WITH_RESERVED_WORDS,
            LANGUAGE_VERSION_WITH_PRIMARY_STATES
        ),
        (15, 13, 15, 14)
    );
    assert_eq!(
        (
            PARSE_ACTION_TYPE_SHIFT,
            PARSE_ACTION_TYPE_REDUCE,
            PARSE_ACTION_TYPE_ACCEPT,
            PARSE_ACTION_TYPE_RECOVER
        ),
        (0, 1, 2, 3)
    );
}

fn no_lex(_: &mut Lexer<'_>, _: StateId) -> bool {
    false
}

#[test]
fn language_tables_are_const_and_clone_keeps_them() {
    static LANG: Language = Language::new(LanguageTables {
        abi_version: 14,
        symbol_count: 1,
        alias_count: 0,
        token_count: 1,
        external_token_count: 0,
        state_count: 1,
        large_state_count: 1,
        production_id_count: 0,
        field_count: 0,
        max_alias_sequence_length: 0,
        parse_table: U16s::EMPTY,
        small_parse_table: U16s::EMPTY,
        small_parse_table_map: U32s::EMPTY,
        parse_actions: ParseActions::EMPTY,
        symbol_names: &["end"],
        field_names: &[""],
        field_map_slices: MapSlices::EMPTY,
        field_map_entries: FieldMapEntries::EMPTY,
        symbol_metadata: SymbolMetadatas::EMPTY,
        public_symbol_map: U16s::EMPTY,
        alias_map: U16s::EMPTY,
        alias_sequences: U16s::EMPTY,
        lex_modes: LexModes::EMPTY,
        lex_fn: no_lex,
        keyword_lex_fn: None,
        keyword_capture_token: 0,
        external_scanner: ExternalScannerInfo {
            states: Bools::EMPTY,
            symbol_map: U16s::EMPTY,
            create: None,
        },
        primary_state_ids: U16s::EMPTY,
        name: "t",
        reserved_words: U16s::EMPTY,
        max_reserved_word_set_size: 0,
        supertype_count: 0,
        supertype_symbols: U16s::EMPTY,
        supertype_map_slices: MapSlices::EMPTY,
        supertype_map_entries: U16s::EMPTY,
        metadata: LanguageMetadata {
            major_version: 0,
            minor_version: 0,
            patch_version: 0,
        },
    });
    let copy = LANG.clone();
    assert_eq!((copy.tables.abi_version, copy.tables.name), (14, "t"));
    let changed = Language::new(LanguageTables {
        abi_version: 12,
        ..LANG.tables
    });
    assert_eq!(changed.tables.abi_version, 12);
}

struct Dummy(u8);

impl ExternalScanner for Dummy {
    fn scan(&mut self, _: &mut Lexer<'_>, _: &[bool]) -> bool {
        false
    }
    fn serialize(&self, buf: &mut [u8; SERIALIZATION_BUFFER_SIZE]) -> usize {
        buf[0] = self.0;
        1
    }
    fn deserialize(&mut self, buf: &[u8]) {
        self.0 = buf.first().copied().unwrap_or(0);
    }
}

#[test]
fn a_kept_scanner_downcasts_to_its_type() {
    // docs/CONTRACT_CHANGES.md 2026-11-13: the reset tests reach the scanner the parser keeps.
    let boxed: Box<dyn ExternalScanner> = Box::new(Dummy(7));
    let scanner: &dyn ExternalScanner = &*boxed;
    let any: &dyn std::any::Any = scanner;
    assert_eq!(any.downcast_ref::<Dummy>().map(|d| d.0), Some(7));
    assert!(any.downcast_ref::<u8>().is_none());
}

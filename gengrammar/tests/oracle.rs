use std::collections::BTreeMap;
use std::path::PathBuf;

use gengrammar::oracle::{OracleData, compare, oracle_from_source, read_oracle_dir};

fn sample() -> OracleData {
    OracleData {
        fields: BTreeMap::from([
            ("ParseActions".to_string(), (0..24).collect::<Vec<u8>>()),
            ("LexModes".to_string(), vec![1, 0, 2, 0, 0, 0]),
            ("ExternalScanner.States".to_string(), vec![]),
        ]),
        symbol_names: vec!["end".into(), "a".into()],
        field_names: vec!["".into(), "f".into()],
        counts: BTreeMap::from([
            ("StateCount".to_string(), "7".to_string()),
            ("Name".to_string(), "json".to_string()),
        ]),
    }
}

fn err(ours: &OracleData, theirs: &OracleData) -> String {
    compare("json", ours, theirs).unwrap_err()
}

#[test]
fn equal_data_is_ok() {
    assert_eq!(compare("json", &sample(), &sample()), Ok(()));
}

#[test]
fn one_byte_differs_in_parse_actions() {
    let mut theirs = sample();
    // byte 10 is in element 1 of 8-byte actions
    theirs.fields.get_mut("ParseActions").unwrap()[10] ^= 1;
    let e = err(&sample(), &theirs);
    assert!(e.contains("json"), "{e}");
    assert!(e.contains("ParseActions"), "{e}");
    assert!(e.contains("element 1"), "{e}");
}

#[test]
fn shorter_table_names_the_first_missing_element() {
    let mut theirs = sample();
    theirs.fields.get_mut("LexModes").unwrap().clear();
    let e = err(&sample(), &theirs);
    assert!(e.contains("LexModes") && e.contains("element 0"), "{e}");
}

#[test]
fn field_only_on_our_side() {
    let mut theirs = sample();
    theirs.fields.remove("LexModes");
    let e = err(&sample(), &theirs);
    assert!(e.contains("LexModes") && e.contains("only"), "{e}");
}

#[test]
fn field_only_on_their_side() {
    let mut ours = sample();
    ours.fields.remove("LexModes");
    let e = err(&ours, &sample());
    assert!(e.contains("LexModes") && e.contains("only"), "{e}");
}

#[test]
fn empty_field_is_not_absent() {
    let mut theirs = sample();
    theirs.fields.remove("ExternalScanner.States");
    assert!(err(&sample(), &theirs).contains("ExternalScanner.States"));
}

#[test]
fn symbol_name_differs() {
    let mut theirs = sample();
    theirs.symbol_names[1] = "b".into();
    let e = err(&sample(), &theirs);
    assert!(e.contains("SymbolNames") && e.contains("element 1"), "{e}");
}

#[test]
fn field_name_differs() {
    let mut theirs = sample();
    theirs.field_names[1] = "g".into();
    let e = err(&sample(), &theirs);
    assert!(e.contains("FieldNames") && e.contains("element 1"), "{e}");
}

#[test]
fn count_differs() {
    let mut theirs = sample();
    theirs.counts.insert("StateCount".into(), "8".into());
    let e = err(&sample(), &theirs);
    assert!(e.contains("StateCount"), "{e}");
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let d = std::env::temp_dir().join(format!(
        "gengrammar-oracle-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn read_oracle_dir_round_trip() {
    let d = temp_dir("rt");
    let want = sample();
    // The Go file layout: one .bin per present field (an empty file is an empty slice),
    // names as u32 LE length + bytes, counts as "Key value" lines.
    for (k, v) in &want.fields {
        std::fs::write(d.join(format!("{k}.bin")), v).unwrap();
    }
    let names = |v: &[String]| -> Vec<u8> {
        v.iter()
            .flat_map(|s| {
                let mut b = (s.len() as u32).to_le_bytes().to_vec();
                b.extend_from_slice(s.as_bytes());
                b
            })
            .collect()
    };
    std::fs::write(d.join("SymbolNames.bin"), names(&want.symbol_names)).unwrap();
    std::fs::write(d.join("FieldNames.bin"), names(&want.field_names)).unwrap();
    std::fs::write(d.join("counts.txt"), "StateCount 7\nName json\n").unwrap();
    // Not a table: ignored.
    std::fs::write(d.join("notes.txt"), "x").unwrap();

    let got = read_oracle_dir(&d).unwrap();
    std::fs::remove_dir_all(&d).unwrap();
    assert_eq!(got, want);
    assert_eq!(got.fields["ExternalScanner.States"], Vec::<u8>::new());
    assert!(!got.fields.contains_key("SmallParseTable"));
}

#[test]
fn read_oracle_dir_missing_dir_is_an_error() {
    let d = temp_dir("missing");
    std::fs::remove_dir_all(&d).unwrap();
    assert!(read_oracle_dir(&d).is_err());
}

#[test]
fn mini_lang_maps_tables_to_go_fields() {
    let o = oracle_from_source(include_str!("../../testdata/gengrammar/mini_lang.c")).unwrap();
    // ts_small_parse_table, as tests/blob.rs reads it
    assert_eq!(
        o.fields["SmallParseTable"][..16],
        [1, 0, 3, 0, 1, 0, 2, 0, 0, 0, 0, 0, 0, 0, 2, 0]
    );
    // ts_lex_modes row 1 and the 0xFFFF row
    assert_eq!(o.fields["LexModes"][6..12], [5, 0, 2, 0, 0, 0]);
    assert_eq!(o.fields["LexModes"][18..24], [0xff, 0xff, 0, 0, 0, 0]);
    // ts_symbol_metadata: three symbols, one byte each
    assert_eq!(o.fields["SymbolMetadata"], [2, 3, 5]);
    // The element is the slot, not the padded blob section.
    assert!(o.fields["ParseActions"].len().is_multiple_of(8));
    assert!(o.fields.contains_key("ExternalScanner.States"));
    assert!(o.fields.contains_key("ExternalScanner.SymbolMap"));
    assert!(!o.fields.contains_key("ts_lex_modes"));
    assert_eq!(o.counts["Name"], "mini");
    assert_eq!(o.symbol_names[0], "end");
    assert_eq!(o.field_names[1], "f");
}

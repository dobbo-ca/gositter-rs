//! The 20 methods graphify-go's adapter (`internal/ts/ts.go:26-77`) calls on gositter, each
//! once on json `{"k": [10, "s"]}`, and `parse_budget(src, 0)` as `parse(src)` (plan Task 5
//! Step 2).

mod common;

use gositter::Parser;

#[test]
fn graphify_api() {
    let src = br#"{"k": [10, "s"]}"#;
    let json = common::language("json").expect("json grammar");
    let mut p = Parser::new();
    p.set_language(json).unwrap();
    let tree = p.parse(src).expect("tree");
    let root = tree.root_node();
    let obj = root.named_child(0).expect("object");
    let pair = obj.named_child(0).expect("pair");

    assert_eq!(pair.kind(), "pair");
    assert!(pair.is_named());
    assert_eq!((pair.start_byte(), pair.end_byte()), (1, 15));
    assert_eq!(pair.start_position().row, 0);
    assert_eq!(pair.child_count(), 3);
    assert_eq!(pair.named_child_count(), 2);
    let key = pair.child(0).expect("key");
    assert_eq!(key.kind(), "string");
    assert_eq!(pair.named_child(1).expect("value").kind(), "array");
    assert_eq!(
        pair.child_by_field_name("value").expect("value").kind(),
        "array"
    );
    assert_eq!(pair.field_name_for_child(0), Some("key"));
    assert!(pair.parent().unwrap().equals(obj));
    assert_eq!(key.utf8_text(src), b"\"k\"");
    let mut cs = Vec::new();
    pair.children(&mut cs);
    assert_eq!(cs.len(), 3);

    let mut visited = 0;
    root.preorder(|_| {
        visited += 1;
        true
    });
    let mut walked = 0;
    let mut c = root.walk();
    'walk: loop {
        walked += 1;
        if c.goto_first_child() {
            continue;
        }
        while !c.goto_next_sibling() {
            if !c.goto_parent() {
                break 'walk;
            }
        }
    }
    assert_eq!(visited, walked);

    let mut pairs = 0;
    root.preorder_mask(json.kind_mask(&["pair"]), |_| {
        pairs += 1;
        true
    });
    assert_eq!(pairs, 1);

    assert_eq!(pair.children_by_field_name("value", &mut c).len(), 1);
    drop(c); // Close

    let want = root.to_sexp();
    let budget = p.parse_budget(src, 0).expect("tree").root_node().to_sexp();
    assert_eq!(budget, want);
}

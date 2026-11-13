//! gositter `example_test.go`: `Example`, with the printed lines compared as a string.

mod common;

use gositter::{Node, Parser};

fn walk(n: Node<'_>, depth: usize, out: &mut String) {
    out.push_str(&format!(
        "{}{} [{}, {})\n",
        "  ".repeat(depth),
        n.kind(),
        n.start_byte(),
        n.end_byte()
    ));
    let mut cs = Vec::new();
    n.named_children(&mut cs);
    for c in cs {
        walk(c, depth + 1, out);
    }
}

#[test]
fn example_python() {
    let src = b"def add(a, b):\n    return a + b\n";
    let mut p = Parser::new();
    p.set_language(common::language("python").expect("python grammar"))
        .unwrap();
    let tree = p.parse(src).expect("tree");
    let mut out = String::new();
    walk(tree.root_node(), 0, &mut out);
    assert_eq!(
        out,
        "module [0, 32)
  function_definition [0, 31)
    identifier [4, 7)
    parameters [7, 13)
      identifier [8, 9)
      identifier [11, 12)
    block [19, 31)
      return_statement [19, 31)
        binary_operator [26, 31)
          identifier [26, 27)
          identifier [30, 31)
"
    );
}

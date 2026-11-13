//! Byte 0 picks a grammar, the rest is the source. The dump of a parse must not change when the
//! parser parsed another input in between, and must carry none of treedump's failed checks.
#![no_main]

#[path = "../common.rs"]
mod fuzz_common;

use fuzz_common::{dump_tree, languages, on_big_stack, parser};
use libfuzzer_sys::fuzz_target;

const OTHER: &[u8] = b"{\"a\": [1, 2.5, true, null, \"x\"]}\nx = f(y)\n";

fn run(data: &[u8]) {
    let Some((&g, src)) = data.split_first() else {
        return;
    };
    let l = languages()[usize::from(g) % languages().len()];
    let mut p = parser(l);
    let first = dump_tree(p.parse(src).unwrap(), l, src);
    p.parse(OTHER).unwrap();
    let again = dump_tree(p.parse(src).unwrap(), l, src);
    assert!(first == again, "the dump changed after another parse");
    for bad in [&b" T!"[..], b"OOB=", b"CURSORKIDS="] {
        assert!(
            !first.windows(bad.len()).any(|w| w == bad),
            "dump has {:?}",
            String::from_utf8_lossy(bad)
        );
    }
}

fuzz_target!(|data: &[u8]| on_big_stack(data, run));

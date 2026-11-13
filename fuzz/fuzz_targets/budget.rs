//! Bytes 0..4 are max_stall (little endian, % 4096 + 1), byte 4 picks a grammar, the rest is the
//! source. parse_budget gives no tree or the tree of parse, and two runs agree.
#![no_main]

#[path = "../common.rs"]
mod fuzz_common;

use fuzz_common::{dump_tree, languages, on_big_stack, parser};
use libfuzzer_sys::fuzz_target;

fn run(data: &[u8]) {
    let Some((head, src)) = data.split_first_chunk::<5>() else {
        return;
    };
    let max_stall = u32::from_le_bytes([head[0], head[1], head[2], head[3]]) % 4096 + 1;
    let l = languages()[usize::from(head[4]) % languages().len()];
    let mut p = parser(l);
    let full = dump_tree(p.parse(src).unwrap(), l, src);
    let a = p.parse_budget(src, max_stall).map(|t| dump_tree(t, l, src));
    let b = p.parse_budget(src, max_stall).map(|t| dump_tree(t, l, src));
    assert!(a == b, "two budget runs differ (max_stall {max_stall})");
    if let Some(a) = a {
        assert!(a == full, "the budget tree differs from the parse tree");
    }
}

fuzz_target!(|data: &[u8]| on_big_stack(data, run));

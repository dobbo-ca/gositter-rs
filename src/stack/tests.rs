//! gositter `stack_test.go`.

use super::*;
use crate::subtree::{ERROR_COST_PER_RECOVERY, ERROR_STATE, NULL_SUBTREE};
use crate::test_language::{TS_ANON, TS_HIDDEN, TS_IDENT, test_language};
use crate::{Language, Point, Symbol};

/// parser.go maxSummaryDepth (MAX_SUMMARY_DEPTH); the parser port will own it.
const MAX_SUMMARY_DEPTH: u32 = 16;

/// newTestStack
fn new_test_stack() -> (Stack, SubtreePool, &'static Language) {
    let mut sp = SubtreePool::default();
    sp.reset();
    let mut s = Stack::default();
    s.init(&mut sp);
    (s, sp, test_language())
}

/// Go's `length{bytes, Point{row, column}}`.
fn len(bytes: u32, row: u32, column: u32) -> Length {
    Length {
        bytes,
        extent: Point { row, column },
    }
}

/// testLeaf
fn test_leaf(sp: &mut SubtreePool, lang: &Language, sym: Symbol, heap: bool) -> Subtree {
    let size = if heap { len(300, 0, 300) } else { len(1, 0, 1) };
    sp.new_leaf(
        sym,
        Length::default(),
        size,
        1,
        1,
        false,
        false,
        false,
        lang,
    )
}

/// TestStackPushPopCount
#[test]
#[ignore = "gositter-rs-2f0.9"]
fn stack_push_pop_count() {
    let (mut s, mut sp, lang) = new_test_stack();
    assert!(
        s.version_count() == 1 && s.state(0) == 1 && s.position(0).bytes == 0,
        "fresh stack"
    );
    let a = test_leaf(&mut sp, lang, TS_ANON, false);
    let b = test_leaf(&mut sp, lang, TS_IDENT, true);
    let c = test_leaf(&mut sp, lang, TS_HIDDEN, false);
    s.push(&sp, 0, a, false, 2);
    s.push(&sp, 0, b, false, 3);
    s.push(&sp, 0, c, false, 4);
    assert!(
        s.state(0) == 4 && s.position(0).bytes == 302,
        "state={} pos={}",
        s.state(0),
        s.position(0).bytes
    );
    // Visible subtrees count as nodes: a and b.
    assert_eq!(s.node_count_since_error(0), 2, "node count");

    let pop = s.pop_count(&mut sp, 0, 2).to_vec();
    assert_eq!(pop.len(), 1, "slices");
    let got = sp.array(pop[0].subtrees);
    assert_eq!(got, &[b, c][..], "popped, want [b c] in stack order");
    // The pop made a new version at the node below; the old one is intact.
    assert!(
        s.version_count() == 2 && pop[0].version == 1 && s.state(1) == 2 && s.state(0) == 4,
        "versions={} slice.version={}",
        s.version_count(),
        pop[0].version
    );
    // The slice holds its own reference to the heap leaf.
    assert_eq!(sp.ptr(b).ref_count, 2, "refcount, want 2");
    s.renumber_version(&mut sp, 1, 0);
    assert!(
        s.version_count() == 1 && s.state(0) == 2 && sp.ptr(b).ref_count == 1,
        "after renumber: versions={} state={} ref={}",
        s.version_count(),
        s.state(0),
        sp.ptr(b).ref_count
    );
}

/// TestStackPopCountSkipsExtras
#[test]
#[ignore = "gositter-rs-2f0.9"]
fn stack_pop_count_skips_extras() {
    let (mut s, mut sp, lang) = new_test_stack();
    let a = test_leaf(&mut sp, lang, TS_ANON, false);
    let mut extra = test_leaf(&mut sp, lang, TS_IDENT, false);
    sp.set_extra(&mut extra, true);
    s.push(&sp, 0, a, false, 2);
    s.push(&sp, 0, extra, false, 2);
    let pop = s.pop_count(&mut sp, 0, 1).to_vec();
    // The extra does not count towards the goal but is part of the slice.
    assert_eq!(sp.array(pop[0].subtrees), &[a, extra][..], "popped");
    let mut trailing = Vec::new();
    let mut arr = pop[0].subtrees;
    sp.array_remove_trailing_extras(&mut arr, &mut trailing);
    assert!(
        arr.size == 1 && trailing == [extra],
        "trailing extras: size={} trailing={:?}",
        arr.size,
        trailing
    );
}

/// TestStackMergeAndAmbiguousPop
#[test]
#[ignore = "gositter-rs-2f0.9"]
fn stack_merge_and_ambiguous_pop() {
    let (mut s, mut sp, lang) = new_test_stack();
    let a = test_leaf(&mut sp, lang, TS_ANON, true);
    let b = test_leaf(&mut sp, lang, TS_IDENT, true);

    // Two versions reach the same state and position over different subtrees.
    let v1 = s.copy_version(&mut sp, 0);
    s.push(&sp, 0, a, false, 5);
    s.push(&sp, v1, b, false, 5);
    assert!(s.can_merge(&sp, 0, v1), "versions should be mergeable");
    assert!(
        s.merge(&mut sp, 0, v1) && s.version_count() == 1,
        "merge failed"
    );
    let n = &s.nodes[s.heads[0].node as usize];
    assert_eq!(n.link_count, 2, "links, want 2");

    // Popping one subtree walks both links. Both paths end at the base node,
    // so both slices carry the same version, first link first.
    let pop = s.pop_count(&mut sp, 0, 1).to_vec();
    assert!(
        pop.len() == 2 && pop[0].version == pop[1].version,
        "slices = {}",
        pop.len()
    );
    assert!(
        sp.array(pop[0].subtrees)[0] == a && sp.array(pop[1].subtrees)[0] == b,
        "slice order does not follow link order"
    );

    // A different state does not merge.
    let v2 = s.copy_version(&mut sp, 0);
    let t = test_leaf(&mut sp, lang, TS_ANON, false);
    s.push(&sp, v2, t, false, 6);
    assert!(!s.can_merge(&sp, 0, v2), "different states must not merge");
}

/// TestStackEquivalentLinksCollapse
#[test]
#[ignore = "gositter-rs-2f0.9"]
fn stack_equivalent_links_collapse() {
    let (mut s, mut sp, lang) = new_test_stack();
    let v1 = s.copy_version(&mut sp, 0);
    // Inline leaves with the same bits are the same subtree value.
    let t = test_leaf(&mut sp, lang, TS_ANON, false);
    s.push(&sp, 0, t, false, 5);
    let t = test_leaf(&mut sp, lang, TS_ANON, false);
    s.push(&sp, v1, t, false, 5);
    assert!(s.merge(&mut sp, 0, v1), "merge failed");
    let n = &s.nodes[s.heads[0].node as usize];
    assert_eq!(n.link_count, 1, "links, want 1");
}

/// TestStackPauseResumeHalt
#[test]
#[ignore = "gositter-rs-2f0.9"]
fn stack_pause_resume_halt() {
    let (mut s, mut sp, lang) = new_test_stack();
    let la = test_leaf(&mut sp, lang, TS_ANON, false);
    let base = s.error_cost(0);
    s.pause(0, la);
    assert!(
        s.is_paused(0) && !s.is_active(0) && s.error_cost(0) == base + ERROR_COST_PER_RECOVERY,
        "pause"
    );
    let got = s.resume(0);
    assert!(got == la && s.is_active(0), "resume");
    s.halt(0);
    assert!(s.is_halted(0), "halt");
}

/// TestStackErrorDiscontinuityAndSummary
#[test]
#[ignore = "gositter-rs-2f0.9"]
fn stack_error_discontinuity_and_summary() {
    let (mut s, mut sp, lang) = new_test_stack();
    let t = test_leaf(&mut sp, lang, TS_ANON, false);
    s.push(&sp, 0, t, false, 2);
    let t = test_leaf(&mut sp, lang, TS_IDENT, false);
    s.push(&sp, 0, t, false, 3);
    s.push(&sp, 0, NULL_SUBTREE, false, ERROR_STATE);
    assert_eq!(s.error_cost(0), ERROR_COST_PER_RECOVERY, "error cost");
    assert_eq!(
        s.node_count_since_error(0),
        0,
        "node count since error should restart at the discontinuity"
    );
    s.record_summary(&mut sp, 0, MAX_SUMMARY_DEPTH);
    // Depth 0 is the error state itself, then states 3, 2 and the base.
    let want = [
        StackSummaryEntry {
            position: len(2, 0, 2),
            depth: 0,
            state: ERROR_STATE,
        },
        StackSummaryEntry {
            position: len(2, 0, 2),
            depth: 1,
            state: 3,
        },
        StackSummaryEntry {
            position: len(1, 0, 1),
            depth: 2,
            state: 2,
        },
        StackSummaryEntry {
            position: Length::default(),
            depth: 3,
            state: 1,
        },
    ];
    // Go's nil summary has length 0, which fails its length check.
    let sum = s.get_summary(0).expect("summary = nil");
    assert_eq!(sum, &want[..], "summary");
    assert_eq!(s.version_count(), 1, "summary walk changed the versions");
}

/// TestStackClearReleases
#[test]
#[ignore = "gositter-rs-2f0.9"]
fn stack_clear_releases() {
    let (mut s, mut sp, lang) = new_test_stack();
    let b = test_leaf(&mut sp, lang, TS_IDENT, true);
    s.push(&sp, 0, b, false, 2);
    s.copy_version(&mut sp, 0);
    s.clear(&mut sp);
    assert!(
        s.version_count() == 1 && s.state(0) == 1 && sp.free_trees.len() == 1,
        "versions={} state={} freed={}",
        s.version_count(),
        s.state(0),
        sp.free_trees.len()
    );
    let base = s.nodes[s.base_node as usize].ref_count;
    assert_eq!(base, 2, "base node refcount, want 2");
}

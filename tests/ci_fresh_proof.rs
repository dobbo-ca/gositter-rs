//! Proof for ci/fresh.sh (gositter-rs-aff): a new test in a tests-only commit must run, so
//! this one must turn the `test` job red. The next commit removes it.

#[test]
fn a_new_test_runs() {
    panic!("ci/fresh.sh proof: this new test ran");
}

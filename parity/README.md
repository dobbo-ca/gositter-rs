# Parity with gositter

The oracles are gositter v0.2.0 (7a27b30), a pure-Go port of the tree-sitter v0.25.0 runtime, and the
C runtime through it:

1. **The C goldens.** `testdata/regress/<dir>/` holds 1,423 cases in 21 grammars: a source file and
   `<file>.dump.gz`, the dump the C runtime gave for it (gositter `oracle dump-c`). They are copied from
   gositter `testdata/regress` and committed gzipped; CI unzips them before the tests run, so gositter-rs
   needs no gzip code. The dump format is gositter's `internal/treedump`, ported in `treedump/`.
2. **The Go dump tool** (plan Task 12): a pure-Go `gsdump`, built with `CGO_ENABLED=0` inside a copy of the
   gositter v0.2.0 module, writes dumps, LOG lines, stall counts and quoted kind names for the regress
   inputs and their mutants; the Rust `gsdump` must write the same bytes. A difference is a bug in the
   port, never in the oracle.

## The parity stages (plan Task 12)

`parity/go/gsdump/main.go` (Go) and `examples/gsdump.rs` (Rust) have the same commands: `list`, `dump`,
`log`, `stall`, `kinds`, and in Go only `gen`. `ci/Dockerfile` runs them in four stages:

| Stage | What it does |
|---|---|
| `go-oracle` | builds the Go `gsdump` as `cmd/gsdump` in a copy of the gositter v0.2.0 module (`CGO_ENABLED=0`; module checked against `parity/go/go.sum`), for amd64 and arm64 |
| `go-run` | the Go dumps of the 1,423 regress inputs must equal the C goldens; then `gsdump gen` writes 16 mutants of each input (8 truncations, 4 line deletions, 2 line swaps, a stray `(` and `{`; seed 1) |
| `parity` | `ci/parity.sh`, both tools on amd64: trees of the regress inputs and all mutants, LOG lines (inputs up to 16 KiB), stall counts (up to 8 KiB), quoted kind names; `diff -r` per dir |
| `parity-arm64` | `ci/parity.sh`, the aarch64 Rust tool and the arm64 Go tool under qemu-user: trees of the regress inputs and every 10th mutant |

A dir that the Rust `gsdump list` does not have yet is printed as skipped. The `parity` and `parity-arm64`
jobs run on main and on `workflow_dispatch`. A LOG difference with equal trees points at the runtime; a
first difference in an `L` line inside an external scan points at the scanner.

Plan: `docs/superpowers/plans/2026-11-13-runtime.md`.

## Count table

| Source | Tests | Ported | N/A-Go | Passing |
|---|---|---|---|---|
| gositter root package `Test` funcs | 53 | 48 | 5 | 29 |
| gositter `Example` / `Benchmark` / `Fuzz` | 3 | 2 | 0 | 1 |
| C goldens (`testdata/regress`) | 1,423 | 1,423 | 0 | 0 |
| gositter `cmd/gengrammar` and `grammars/internal/cchar` tests | 3 | 3 | 0 | 3 |
| gositter `grammars/<g>/scanner_test.go` and `reset_test.go` (62 compiled, 1 compiled out) | 63 | 63 | 0 | 42 |

"Ported" counts a test once it is in the tree (passing or `#[ignore = "<bead>"]`); "Passing" counts it once
it runs green in the `test` job.

## Cases per grammar

| Dir | Cases | Dir | Cases | Dir | Cases |
|---|---|---|---|---|---|
| bash | 65 | julia | 68 | rust | 85 |
| c | 50 | kotlin | 79 | scala | 78 |
| c-sharp | 73 | lua | 80 | tsx | 78 |
| cpp | 85 | php | 66 | typescript | 65 |
| go | 51 | python | 63 | verilog | 61 |
| hcl | 71 | ruby | 70 | zig | 47 |
| java | 53 | javascript | 85 | json | 50 |

`tests/regress.rs` `corpus_is_complete_and_unzipped` checks these counts.

## gositter root package tests

| Go file | Go test | Rust test | Rust file | Status |
|---|---|---|---|---|
| lexer_test.go | `TestLexerDecode` | `lexer_decode` | `src/lexer.rs` | passing |
| lexer_test.go | `TestLexerAdvanceAfterEOF` | `lexer_advance_after_eof` | `src/lexer.rs` | passing |
| lexer_test.go | `TestLexerEmptyInput` | `lexer_empty_input` | `src/lexer.rs` | passing |
| lexer_test.go | `TestLexerBOMAndPositions` | `lexer_bom_and_positions` | `src/lexer.rs` | passing |
| lexer_test.go | `TestLexerIsAtIncludedRangeStart` | `lexer_is_at_included_range_start` | `src/lexer.rs` | passing |
| lexer_test.go | `TestLexerFinish` | `lexer_finish` | `src/lexer.rs` | passing |
| lexer_test.go | `TestLexerResetBackwards` | `lexer_reset_backwards` | `src/lexer.rs` | passing |
| lexer_test.go | `TestAdvanceRun` | `advance_run` | `src/lexer.rs` | passing |
| subtree_test.go | `TestSubtreeInlineLayout` | `subtree_inline_layout` | `src/subtree.rs` | passing |
| subtree_test.go | `TestSubtreeCanInline` | `subtree_can_inline` | `src/subtree.rs` | passing |
| subtree_test.go | `TestSubtreeMissingAndErrorCost` | `subtree_missing_and_error_cost` | `src/subtree.rs` | passing |
| subtree_test.go | `TestSubtreeNewNodeSummary` | `subtree_new_node_summary` | `src/subtree.rs` | passing |
| subtree_test.go | `TestSubtreeMakeMutAndRelease` | `subtree_make_mut_and_release` | `src/subtree.rs` | passing |
| subtree_test.go | `TestSubtreeCompare` | `subtree_compare` | `src/subtree.rs` | passing |
| subtree_test.go | `TestSubtreeArraySplice` | `subtree_array_splice` | `src/subtree.rs` | passing |
| subtree_test.go | `TestSubtreeRepeatDepthOfLeaves` | `subtree_repeat_depth_of_leaves` | `src/subtree.rs` | passing |
| subtree_test.go | `TestChildClass` | `child_class` | `src/subtree.rs` | passing |
| subtree_test.go | `TestChildrenBlockReuse` | `children_block_reuse` | `src/subtree.rs` | passing |
| subtree_test.go | `TestChildrenAllocator` | `children_allocator` | `src/subtree.rs` | passing |
| subtree_test.go | `TestSummarizeRandom` | `summarize_random` | `src/subtree.rs` | passing |
| subtree_test.go | `TestSummarizeResume` | `summarize_resume` | `src/subtree.rs` | passing |
| subtree_test.go | `TestCompressRandom` | `compress_random` | `src/subtree.rs` | passing |
| subtree_test.go | `TestColdGrowthAsMain` | — | — | N/A-Go: Go GC and append growth |
| stack_test.go | `TestStackPushPopCount` | `stack_push_pop_count` | `src/stack.rs` | passing |
| stack_test.go | `TestStackPopCountSkipsExtras` | `stack_pop_count_skips_extras` | `src/stack.rs` | passing |
| stack_test.go | `TestStackMergeAndAmbiguousPop` | `stack_merge_and_ambiguous_pop` | `src/stack.rs` | passing |
| stack_test.go | `TestStackEquivalentLinksCollapse` | `stack_equivalent_links_collapse` | `src/stack.rs` | passing |
| stack_test.go | `TestStackPauseResumeHalt` | `stack_pause_resume_halt` | `src/stack.rs` | passing |
| stack_test.go | `TestStackErrorDiscontinuityAndSummary` | `stack_error_discontinuity_and_summary` | `src/stack.rs` | passing |
| stack_test.go | `TestStackClearReleases` | `stack_clear_releases` | `src/stack.rs` | passing |
| dense_test.go | `TestDenseLookup` | `dense_lookup` | `tests/dense.rs` | ported, ignored until its grammar |
| parser_test.go | `TestParseJSONString` | `parse_json_string` | `tests/parser.rs` | ported, ignored until its grammar |
| parser_test.go | `TestNodeAPI` | `node_api` | `tests/parser.rs` | ported, ignored until its grammar |
| parser_test.go | `TestParserReuseAndNoLanguage` | `parser_reuse_and_no_language` | `tests/parser.rs` | ported, ignored until its grammar |
| parser_test.go | `TestLoggerMessages` | `logger_messages` | `tests/parser.rs` | ported, ignored until its grammar |
| parser_test.go | `TestParseAllocs` | — | — | N/A-Go: AllocsPerRun |
| parser_test.go | `TestErrorRecoveryMemory` | `error_recovery_memory` | `tests/parser.rs` | ported, ignored until its grammar; alloc half N/A-Go |
| parser_test.go | `TestHeapChunksShared` | `heap_chunks_reused` | `tests/parser.rs` | ported, ignored until its grammar; shared-pool half N/A-Go |
| parser_test.go | `TestChildrenSlabDropped` | `children_slab_dropped` | `tests/parser.rs` | ported, ignored until its grammar; alloc half N/A-Go |
| parser_test.go | `TestParseBudgetStops` | `parse_budget_stops` | `tests/parser.rs` | ported, ignored until its grammar |
| parser_test.go | `TestParseBudgetBoundary` | `parse_budget_boundary` | `tests/parser.rs` | ported, ignored until its grammar |
| parser_test.go | `TestParseBudgetCountStable` | `parse_budget_count_stable` | `tests/parser.rs` | ported, ignored until its grammar |
| parser_test.go | `TestStackSlabsDropped` | `stack_slabs_dropped` | `tests/parser.rs` | ported, ignored until its grammar; alloc half N/A-Go |
| parser_test.go | `TestHugeSlabsFreed` | — | — | N/A-Go: finalizers, weak pointers |
| parser_test.go | `TestKeptLinkSlabOutlivesGC` | — | — | N/A-Go: weak pointers |
| parser_test.go | `TestHugeLinkSlabFreedByLinklessParses` | — | — | N/A-Go: weak pointers |
| parser_test.go | `TestSmallParseTakesSmallSlabs` | `small_parse_takes_small_slabs` | `tests/parser.rs` | ported, ignored until its grammar |
| parser_test.go | `TestSlabsAcrossParsers` | `slabs_across_parsers` | `tests/parser.rs` | ported (threads), ignored until its grammar |
| scanner_reuse_test.go | `TestScannerReuseAcrossLanguages` | `scanner_reuse_across_languages` | `tests/scanner_reuse.rs` | ported, ignored until its grammar |
| scanner_reuse_test.go | `TestSetLanguageDropsKeptScanner` | `set_language_drops_kept_scanner` | `tests/scanner_reuse.rs` | ported, ignored until its grammar |
| regress_test.go | `TestRegress` | `regress_<dir> x21` | `tests/regress.rs` | ported, ignored until its grammar |
| regress_test.go | `TestSummaries` | `summaries` | `tests/regress.rs` | ported, ignored until its grammar |
| regress_test.go | `TestChildrenWide` | `children_wide` | `tests/regress.rs` | ported, ignored until its grammar |

## Example, Benchmark, Fuzz

| Go file | Go test | Rust test | Rust file | Status |
|---|---|---|---|---|
| example_test.go | `Example` | `example_python` | `tests/example.rs` | ported, ignored until its grammar |
| parser_test.go | `BenchmarkParseJSON` | `gsdump bench json_2000` | `examples/gsdump.rs` | to port (Task 14) |
| lexer_test.go | `FuzzAdvanceRun` | `advance_run_random + fuzz target advance_run` | `src/lexer.rs, fuzz/` | `advance_run_random` passing; fuzz target in Task 13 |

## Grammar tests

Plan: `docs/superpowers/plans/2026-11-13-grammars.md` (grammar stream). Scanner tests are unit tests in
`grammars/<g>/scanner.rs`. A test that is not passing yet is either compiled and
`#[ignore = "<bead>"]` (it compiles against the runtime's stubbed API; the grammar's `language()` is a
test-only stand-in until Task 10 generates it), or compiled out with `#[cfg(any())]` because it names
something that does not exist yet (a comment gives the bead). "Ported" is the sum of the three counts.

| Source file | Tests | Passing | Compiled + ignored | Compiled out |
|---|---|---|---|---|
| `cmd/gengrammar/gengrammar_test.go` | 2 | 2 | 0 | 0 |
| `grammars/internal/cchar/cchar_test.go` | 1 | 1 | 0 | 0 |
| `bash/scanner_test.go` | 4 | 3 | 1 `TestHeredocEndChar` | 0 |
| `bash/reset_test.go` | 3 | 0 | 3 `TestResetIsNew`, `TestReusedScannerSameTree`, `TestResetDropsBigBuffers` | 0 |
| `cpp/scanner_test.go` | 4 | 3 | 1 `TestRawStrings` | 0 |
| `cpp/reset_test.go` | 2 | 0 | 2 `TestResetIsNew`, `TestReusedScannerSameTree` | 0 |
| `csharp/scanner_test.go` | 5 | 4 | 1 `TestScanState` | 0 |
| `hcl/scanner_test.go` | 3 | 3 | 0 | 0 |
| `hcl/reset_test.go` | 3 | 0 | 3 `TestResetIsNew`, `TestReusedScannerSameTree`, `TestResetDropsBigBuffers` | 0 |
| `javascript/scanner_test.go` | 4 | 2 | 1 `TestScanSmoke` | 1 `TestTokenOrder` (`.10`: needs the generated consts) |
| `julia/scanner_test.go` | 3 | 1 | 2 `TestLanguageWiresScanner`, `TestParseScannerTokens` | 0 |
| `kotlin/scanner_test.go` | 3 | 2 | 1 `TestScan` | 0 |
| `lua/scanner_test.go` | 3 | 2 | 1 `TestLongBrackets` | 0 |
| `php/scanner_test.go` | 4 | 4 | 0 | 0 |
| `python/scanner_test.go` | 3 | 3 | 0 | 0 |
| `python/reset_test.go` | 2 | 0 | 2 `TestResetIsNew`, `TestReusedScannerSameTree` | 0 |
| `ruby/scanner_test.go` | 5 | 5 | 0 | 0 |
| `rust/scanner_test.go` | 2 | 2 | 0 | 0 |
| `scala/scanner_test.go` | 4 | 4 | 0 | 0 |
| `tsx/scanner_test.go` | 3 | 2 | 1 `TestParse` | 0 |
| `typescript/scanner_test.go` | 3 | 2 | 1 `TestParse` | 0 |
| **Total** | **66** | **45** | **20** | **1** |

Beads: `.10` is `gositter-rs-vjz.10` (Task 10, generated `language()`), `.12` is `gositter-rs-vjz.12`
(Task 12, `scan` and the `ExternalScanner` impl), `gositter-rs-2f0.11` is the runtime parser (the
reset tests read `Parser::kept_scanner`, which is `todo!` until it lands). The hand-made half of bash `TestResetIsNew` already runs
as `reset_hand_made`. Tests with no Go source are not counted: gengrammar's SHA-256 vectors, lock file,
tokenizer and parser rejects, and the oracle diffs of Tasks 6 and 8, which run in addition to the ported
tests.

## Tests added by the port (not in gositter)

| Rust test | File | Status |
|---|---|---|
| `regress_json`, `error_recovery_memory_json`, `summaries_json`, `children_wide_json`, `dense_lookup_json` | `tests/` | json halves of the Go tests above, ignored until json is wired (gositter-rs-2f0.11) |
| `parse_budget_boundary_json`, `parse_budget_count_stable_json`, `parse_budget_stops_json` | `tests/parser.rs` | json twins of the budget tests (plan Task 11), ignored (gositter-rs-2f0.11) |
| `log_lines_cut_at_1023`, `kind_mask_after_parse` | `tests/parser.rs` | plan Task 11 and Review Focus 1, ignored (gositter-rs-2f0.11) |
| `graphify_api` | `tests/graphify_api.rs` | graphify's 20 adapter methods, ignored (gositter-rs-2f0.11) |
| `kind_mask_threads` and the other `language::tests` | `src/language.rs` | passing |
| `node_api_built_tree`, `cursor_built_tree`, `preorder_deep_small_stack` | `src/node/tests.rs` | Task 10b |
| `go_quote_cases`, `dump_fake_tree`, `sexp_only_up_to_depth_100`, `sampled_rule` | `treedump/tests/dump.rs` | passing |

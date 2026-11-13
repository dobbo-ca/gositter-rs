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

Plan: `docs/superpowers/plans/2026-11-13-runtime.md`.

## Count table

| Source | Tests | Ported | N/A-Go | Passing |
|---|---|---|---|---|
| gositter root package `Test` funcs | 53 | 0 | 5 | 0 |
| gositter `Example` / `Benchmark` / `Fuzz` | 3 | 0 | 0 | 0 |
| C goldens (`testdata/regress`) | 1,423 | 0 | 0 | 0 |

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
| lexer_test.go | `TestLexerDecode` | `lexer_decode` | `src/lexer.rs` | to port (Task 6) |
| lexer_test.go | `TestLexerAdvanceAfterEOF` | `lexer_advance_after_eof` | `src/lexer.rs` | to port (Task 6) |
| lexer_test.go | `TestLexerEmptyInput` | `lexer_empty_input` | `src/lexer.rs` | to port (Task 6) |
| lexer_test.go | `TestLexerBOMAndPositions` | `lexer_bom_and_positions` | `src/lexer.rs` | to port (Task 6) |
| lexer_test.go | `TestLexerIsAtIncludedRangeStart` | `lexer_is_at_included_range_start` | `src/lexer.rs` | to port (Task 6) |
| lexer_test.go | `TestLexerFinish` | `lexer_finish` | `src/lexer.rs` | to port (Task 6) |
| lexer_test.go | `TestLexerResetBackwards` | `lexer_reset_backwards` | `src/lexer.rs` | to port (Task 6) |
| lexer_test.go | `TestAdvanceRun` | `advance_run` | `src/lexer.rs` | to port (Task 6) |
| subtree_test.go | `TestSubtreeInlineLayout` | `subtree_inline_layout` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestSubtreeCanInline` | `subtree_can_inline` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestSubtreeMissingAndErrorCost` | `subtree_missing_and_error_cost` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestSubtreeNewNodeSummary` | `subtree_new_node_summary` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestSubtreeMakeMutAndRelease` | `subtree_make_mut_and_release` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestSubtreeCompare` | `subtree_compare` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestSubtreeArraySplice` | `subtree_array_splice` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestSubtreeRepeatDepthOfLeaves` | `subtree_repeat_depth_of_leaves` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestChildClass` | `child_class` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestChildrenBlockReuse` | `children_block_reuse` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestChildrenAllocator` | `children_allocator` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestSummarizeRandom` | `summarize_random` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestSummarizeResume` | `summarize_resume` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestCompressRandom` | `compress_random` | `src/subtree.rs` | to port (Task 8) |
| subtree_test.go | `TestColdGrowthAsMain` | — | — | N/A-Go: Go GC and append growth |
| stack_test.go | `TestStackPushPopCount` | `stack_push_pop_count` | `src/stack.rs` | to port (Task 9) |
| stack_test.go | `TestStackPopCountSkipsExtras` | `stack_pop_count_skips_extras` | `src/stack.rs` | to port (Task 9) |
| stack_test.go | `TestStackMergeAndAmbiguousPop` | `stack_merge_and_ambiguous_pop` | `src/stack.rs` | to port (Task 9) |
| stack_test.go | `TestStackEquivalentLinksCollapse` | `stack_equivalent_links_collapse` | `src/stack.rs` | to port (Task 9) |
| stack_test.go | `TestStackPauseResumeHalt` | `stack_pause_resume_halt` | `src/stack.rs` | to port (Task 9) |
| stack_test.go | `TestStackErrorDiscontinuityAndSummary` | `stack_error_discontinuity_and_summary` | `src/stack.rs` | to port (Task 9) |
| stack_test.go | `TestStackClearReleases` | `stack_clear_releases` | `src/stack.rs` | to port (Task 9) |
| dense_test.go | `TestDenseLookup` | `dense_lookup` | `tests/dense.rs` | to port (Task 5) |
| parser_test.go | `TestParseJSONString` | `parse_json_string` | `tests/parser.rs` | to port (Task 5) |
| parser_test.go | `TestNodeAPI` | `node_api` | `tests/parser.rs` | to port (Task 5) |
| parser_test.go | `TestParserReuseAndNoLanguage` | `parser_reuse_and_no_language` | `tests/parser.rs` | to port (Task 5) |
| parser_test.go | `TestLoggerMessages` | `logger_messages` | `tests/parser.rs` | to port (Task 5) |
| parser_test.go | `TestParseAllocs` | — | — | N/A-Go: AllocsPerRun |
| parser_test.go | `TestErrorRecoveryMemory` | `error_recovery_memory` | `tests/parser.rs` | to port (Task 5); alloc half N/A-Go |
| parser_test.go | `TestHeapChunksShared` | `heap_chunks_reused` | `tests/parser.rs` | to port (Task 5); shared-pool half N/A-Go |
| parser_test.go | `TestChildrenSlabDropped` | `children_slab_dropped` | `tests/parser.rs` | to port (Task 5); alloc half N/A-Go |
| parser_test.go | `TestParseBudgetStops` | `parse_budget_stops` | `tests/parser.rs` | to port (Task 5) |
| parser_test.go | `TestParseBudgetBoundary` | `parse_budget_boundary` | `tests/parser.rs` | to port (Task 5) |
| parser_test.go | `TestParseBudgetCountStable` | `parse_budget_count_stable` | `tests/parser.rs` | to port (Task 5) |
| parser_test.go | `TestStackSlabsDropped` | `stack_slabs_dropped` | `tests/parser.rs` | to port (Task 5); alloc half N/A-Go |
| parser_test.go | `TestHugeSlabsFreed` | — | — | N/A-Go: finalizers, weak pointers |
| parser_test.go | `TestKeptLinkSlabOutlivesGC` | — | — | N/A-Go: weak pointers |
| parser_test.go | `TestHugeLinkSlabFreedByLinklessParses` | — | — | N/A-Go: weak pointers |
| parser_test.go | `TestSmallParseTakesSmallSlabs` | `small_parse_takes_small_slabs` | `tests/parser.rs` | to port (Task 5) |
| parser_test.go | `TestSlabsAcrossParsers` | `slabs_across_parsers` | `tests/parser.rs` | to port (Task 5), threads |
| scanner_reuse_test.go | `TestScannerReuseAcrossLanguages` | `scanner_reuse_across_languages` | `tests/scanner_reuse.rs` | to port (Task 5) |
| scanner_reuse_test.go | `TestSetLanguageDropsKeptScanner` | `set_language_drops_kept_scanner` | `tests/scanner_reuse.rs` | to port (Task 5) |
| regress_test.go | `TestRegress` | `regress_<dir> x21` | `tests/regress.rs` | to port (Task 5) |
| regress_test.go | `TestSummaries` | `summaries` | `tests/regress.rs` | to port (Task 5) |
| regress_test.go | `TestChildrenWide` | `children_wide` | `tests/regress.rs` | to port (Task 5) |

## Example, Benchmark, Fuzz

| Go file | Go test | Rust test | Rust file | Status |
|---|---|---|---|---|
| example_test.go | `Example` | `example_python` | `tests/example.rs` | to port (Task 5) |
| parser_test.go | `BenchmarkParseJSON` | `gsdump bench json_2000` | `examples/gsdump.rs` | to port (Task 14) |
| lexer_test.go | `FuzzAdvanceRun` | `advance_run_random + fuzz target advance_run` | `src/lexer.rs, fuzz/` | to port (Tasks 6, 13) |

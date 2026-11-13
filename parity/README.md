# Parity

Test counts per source: tests in the Go source, tests ported to Rust, and ported tests that pass on main. A parser test ported before the runtime parser exists is counted as ported and `#[ignore = "<bead>"]`, not passing.

## Grammars (gengrammar, tables, scanners)

Source: gositter v0.2.0 (7a27b30). Plan: `docs/superpowers/plans/2026-11-13-grammars.md`.

| Source file | Source | Ported | Passing |
|---|---|---|---|
| `cmd/gengrammar/gengrammar_test.go` | 2 | 0 | 0 |
| `grammars/internal/cchar/cchar_test.go` | 1 | 0 | 0 |
| `grammars/bash/scanner_test.go` | 4 | 0 | 0 |
| `grammars/bash/reset_test.go` | 3 | 0 | 0 |
| `grammars/cpp/scanner_test.go` | 4 | 0 | 0 |
| `grammars/cpp/reset_test.go` | 2 | 0 | 0 |
| `grammars/csharp/scanner_test.go` | 5 | 0 | 0 |
| `grammars/hcl/scanner_test.go` | 3 | 0 | 0 |
| `grammars/hcl/reset_test.go` | 3 | 0 | 0 |
| `grammars/javascript/scanner_test.go` | 4 | 0 | 0 |
| `grammars/julia/scanner_test.go` | 3 | 0 | 0 |
| `grammars/kotlin/scanner_test.go` | 3 | 0 | 0 |
| `grammars/lua/scanner_test.go` | 3 | 0 | 0 |
| `grammars/php/scanner_test.go` | 4 | 0 | 0 |
| `grammars/python/scanner_test.go` | 3 | 0 | 0 |
| `grammars/python/reset_test.go` | 2 | 0 | 0 |
| `grammars/ruby/scanner_test.go` | 5 | 0 | 0 |
| `grammars/rust/scanner_test.go` | 2 | 0 | 0 |
| `grammars/scala/scanner_test.go` | 4 | 0 | 0 |
| `grammars/tsx/scanner_test.go` | 3 | 0 | 0 |
| `grammars/typescript/scanner_test.go` | 3 | 0 | 0 |
| **Total** | **66** | **0** | **0** |

Tests with no Go source (gengrammar's SHA-256 vectors, lock file, tokenizer and parser rejects) are not counted here.

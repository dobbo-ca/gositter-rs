# gositter-rs

A pure-Rust port of [gositter](https://github.com/dobbo-ca/gositter) v0.2.0 (7a27b30), which is a pure-Go port of the [tree-sitter](https://github.com/tree-sitter/tree-sitter) v0.25.0 runtime, with all 21 grammars gositter ships. The crate is `gositter`. It is the parser under [graphify-rs](https://github.com/dobbo-ca/graphify-rs).

Status: P0 (CI skeleton). Design: [graphify-rs design](https://github.com/dobbo-ca/graphify-rs/blob/main/docs/superpowers/specs/2026-11-12-graphify-rs-design.md).

## Parity

Trees must be C-exact, as in gositter's CONTRACT.md: the 1,423 C-made goldens in `testdata/regress` and the gositter (Go) dump tool are the oracles.

## No C

No `cc`, `bindgen`, `-sys` crate or FFI to C. `deny.toml` bans `cc`, `cmake`, `bindgen`, `ring`, `aws-lc-*`, `openssl-sys`, `security-framework-sys` and `core-foundation-sys`; the `test` CI job runs `cargo deny check`. `unsafe` is allowed only in SIMD and hot kernels, each with a scalar oracle and a `<CRATE>_SIMD=scalar` override.

## Third-party crates

None.

## Differences from the dobbo-ca/rust-ports conventions

| Rule | Here | Reason |
|---|---|---|
| Repos private | Public | Owner decision, 2026-11-12. |
| Tracked in the rust-ports checklist | Tracked in this repo's beads | Owner decision: rust-ports is for image libraries only. |
| CI on ARC only | ARC, plus GitHub-hosted macOS for darwin test and release jobs (from P2) | ARC is Linux only and darwin binaries ship. |

## Building

CI builds everything: see `ci/Dockerfile` and `.github/workflows/ci.yml` (BuildKit on the ARC runners). A PR needs the `test` job; `arm64` runs on main.

## Issue tracking

This repo uses [beads](https://github.com/steveyegge/beads): `bd ready`, `bd update <id> --claim`, `bd close <id>`, then `bd dolt push`. A fresh clone gets the issues with `bd dolt pull`.

## Licence

MIT. See `LICENSE` and `LICENSE-tree-sitter`.

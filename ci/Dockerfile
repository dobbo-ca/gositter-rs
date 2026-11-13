# CI image: Rust runs here, on the in-cluster BuildKit, because the ARC runner has no toolchain.
# Copied from dobbo-ca/deflate-rs ci/Dockerfile. Every tool is installed before `COPY . .`, so a
# new commit does not install it again.
FROM rust:1.99.1-bookworm AS toolchain
RUN rustup component add rustfmt clippy && rustup target add aarch64-unknown-linux-gnu
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install cargo-deny --locked --version ~0.18
WORKDIR /src

FROM toolchain AS src
COPY . .
# The regress goldens are committed gzipped; the tests read them unzipped (gositter-rs needs no gzip code).
RUN find testdata/regress -name '*.dump.gz' -exec gunzip -k {} +

# /src/target mounts are sharing=private: a build that finds the cache in use gets its own copy
# (new or older) instead of waiting for it (gositter-rs-768). ci/fresh.sh makes the sources newer
# than any artifact in such a copy, so it cannot give a stale green.
# --- Grammar tables (docs/superpowers/plans/2026-11-13-grammars.md, Task 5) ---

# The parser.c of each grammar module in grammars.lock, from the Go module proxy, in the Go module
# cache layout (<module>@<version>/<dir>/parser.c). Only the lock is copied, so BuildKit keeps this
# layer until the lock changes and a normal build does not download again. gengrammar checks each
# file's sha256 against the lock; a mismatch fails the build.
FROM toolchain AS gen-src
RUN apt-get update && apt-get install -y --no-install-recommends unzip && rm -rf /var/lib/apt/lists/*
COPY grammars/grammars.lock /gen/grammars.lock
RUN set -e; grep -v '^#' /gen/grammars.lock | while read -r pkg module version dir sum; do \
        [ -n "$pkg" ] || continue; \
        curl -fsSL --retry 3 "https://proxy.golang.org/$module/@v/$version.zip" -o /gen/m.zip; \
        unzip -q -o /gen/m.zip "$module@$version/$dir/parser.c" -d /parsers; \
    done; rm /gen/m.zip; test "$(find /parsers -name parser.c | wc -l)" -eq 21

# gengrammar over all 21 grammars. Its own target dir and no cache mount: the build is small, and a
# fresh build cannot reuse a stale binary from the shared target cache.
FROM src AS gen
COPY --from=gen-src /parsers /parsers
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    CARGO_TARGET_DIR=/gen-target cargo build --locked --release -p gengrammar \
    && /gen-target/release/gengrammar --lock grammars/grammars.lock --src /parsers --out /out -j 4

# The generated files only, for `outputs: type=local` (.github/workflows/gen.yml).
FROM scratch AS gen-out
COPY --from=gen /out /

# The checks run in the build, so a red check fails the build; a rerun of the same source is a
# BuildKit cache hit.
FROM src AS ci
# fmt reads only the sources: no cache mount, so it never waits for one.
RUN cargo fmt --all --check
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target,sharing=private \
    sh ci/fresh.sh && cargo clippy --locked --workspace --all-targets -- -D warnings
# aarch64-only code (NEON kernels, unsigned C char) is not compiled for x86_64, so lint it too.
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target,sharing=private \
    sh ci/fresh.sh && cargo clippy --locked --workspace --all-targets --target aarch64-unknown-linux-gnu -- -D warnings
# No C in the build graph: the ban list in deny.toml.
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    cargo deny --locked check
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target,sharing=private \
    sh ci/fresh.sh && cargo test --locked --workspace

# qemu-user and the aarch64 cross toolchain, for the arm64 stage.
FROM toolchain AS qemu-tools
RUN apt-get update && apt-get install -y --no-install-recommends qemu-user gcc-aarch64-linux-gnu libc6-dev-arm64-cross \
    && rm -rf /var/lib/apt/lists/*
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="qemu-aarch64 -L /usr/aarch64-linux-gnu"

FROM qemu-tools AS qemu-arm64
COPY . .
RUN find testdata/regress -name '*.dump.gz' -exec gunzip -k {} +

# The full test suite for aarch64 under qemu-user.
FROM qemu-arm64 AS arm64
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target,sharing=private \
    sh ci/fresh.sh && cargo test --locked --workspace --target aarch64-unknown-linux-gnu

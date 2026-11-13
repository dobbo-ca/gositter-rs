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

# The checks run in the build, so a red check fails the build; a rerun of the same source is a
# BuildKit cache hit.
FROM src AS ci
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo fmt --all --check
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo clippy --locked --workspace --all-targets -- -D warnings
# aarch64-only code (NEON kernels, unsigned C char) is not compiled for x86_64, so lint it too.
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo clippy --locked --workspace --all-targets --target aarch64-unknown-linux-gnu -- -D warnings
# No C in the build graph: the ban list in deny.toml.
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    cargo deny --locked check
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo test --locked --workspace

# qemu-user and the aarch64 cross toolchain, for the arm64 stage.
FROM toolchain AS qemu-tools
RUN apt-get update && apt-get install -y --no-install-recommends qemu-user gcc-aarch64-linux-gnu libc6-dev-arm64-cross \
    && rm -rf /var/lib/apt/lists/*
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="qemu-aarch64 -L /usr/aarch64-linux-gnu"

FROM qemu-tools AS qemu-arm64
COPY . .

# The full test suite for aarch64 under qemu-user.
FROM qemu-arm64 AS arm64
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo test --locked --workspace --target aarch64-unknown-linux-gnu

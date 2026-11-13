#!/bin/sh
# Cargo decides freshness by mtime. The /src/target cache mount can hold artifacts that another
# build wrote after this checkout's sources, and cargo would then reuse a stale binary (a false
# green: run 43367559834). Touching every source makes the workspace crates always rebuild;
# registry dependencies stay cached.
set -e
find /src -path /src/target -prune -o -type f -exec touch -c {} +

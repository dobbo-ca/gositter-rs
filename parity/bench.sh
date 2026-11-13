#!/bin/sh
# Parse time, Go and Rust gsdump on the same inputs, as a markdown table (plan Task 14):
# sh parity/bench.sh [reps]. Needs gsdump-go and gsdump-rs on PATH; run from the repo root.
# Inputs: the regress sources of every grammar the Rust gsdump has, and json_2000 (the input of
# gositter BenchmarkParseJSON). The spec gate, Rust at most 1.10x of Go, is printed, not enforced.
set -eu
export LC_ALL=C
reps=${1:-20}
grep -m1 'model name' /proc/cpuinfo >&2 || true
mkdir -p /tmp/bench/json_2000
awk 'BEGIN { printf "["; for (i = 0; i < 2000; i++) printf "{\"a\": [1, 2.5, true, null, \"x\\n\"], \"b\": {\"c\": []}},"; printf "0]" }' \
  > /tmp/bench/json_2000/json_2000
# Best of five runs per file and side: the machine is shared, and one run can be slow.
best() {
  awk -F'\t' '!($1 in m) || $3 < m[$1] { m[$1] = $3; b[$1] = $2 } END { for (k in m) printf "%s\t%s\t%s\n", k, b[k], m[k] }' "$@" | sort
}
# One row per grammar: name, files, bytes, Go ns, Rust ns.
row() { # <row name> <grammar> <input dir>
  for i in 1 2 3 4 5; do
    gsdump-go bench "$2" "$3" "$reps" > /tmp/go$i.tsv
    gsdump-rs bench "$2" "$3" "$reps" > /tmp/rs$i.tsv
  done
  best /tmp/go?.tsv > /tmp/go.tsv
  best /tmp/rs?.tsv > /tmp/rs.tsv
  join -t "$(printf '\t')" /tmp/go.tsv /tmp/rs.tsv \
    | awk -F'\t' -v k="$1" '{ c++; b += $2; g += $3; r += $5 } END { printf "%s\t%d\t%d\t%d\t%d\n", k, c, b, g, r }'
}
{
  for d in $(gsdump-rs list); do row "$d" "$d" "testdata/regress/$d"; done
  if gsdump-rs list | grep -qx json; then row json_2000 json /tmp/bench/json_2000; fi
} > /tmp/rows.tsv
# ns per byte is ms per MB.
awk -F'\t' '
function line(k, c, b, g, r) {
  printf "| %s | %d | %.2f | %.3f | %.3f | %.2f | %s |\n", k, c, b / 1e6, g / b, r / b, r / g, r <= 1.10 * g ? "ok" : "over 1.10x"
}
BEGIN { print "| grammar | files | MB | Go ms/MB | Rust ms/MB | Rust / Go | gate (1.10x) |"; print "|---|---|---|---|---|---|---|" }
{ line($1, $2, $3, $4, $5); if ($1 != "json_2000") { c += $2; b += $3; g += $4; r += $5 } }
END { if (b) line("all regress", c, b, g, r) }' /tmp/rows.tsv

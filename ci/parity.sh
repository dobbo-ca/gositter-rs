#!/bin/sh
# The parity check (plan Task 12): the Go and the Rust gsdump over testdata/regress and the
# mutants in <extra>, for every dir the Rust gsdump has; any byte difference fails. With "full",
# also the LOG lines (inputs up to 16 KiB), the stall counts (up to 8 KiB) and the kind names.
# A command can carry a prefix (qemu-aarch64), so $go and $rs are not quoted.
# Usage: sh ci/parity.sh <go-gsdump> <rust-gsdump> <extra> [full]
go=$1 rs=$2 extra=$3 full=$4 bad=0
list=$($rs list) || exit 1
have=" $(echo $list) "

# One command of one side; a failure is named (exit status, and the last file it wrote).
step() {
    side=$1 what=$2 dir=$3
    shift 3
    "$@" || { echo "parity: $side $what $d failed (exit $?), last output: $(ls $dir 2>/dev/null | tail -1)"; bad=1; }
}

run() {
    side=$1 o=/parity-out/$1/$d
    shift
    step $side dump $o/regress "$@" dump $d testdata/regress/$d $o/regress
    step $side dump-extra $o/extra "$@" dump $d $extra/$d $o/extra
    if [ -n "$full" ]; then
        step $side log $o/log "$@" log $d testdata/regress/$d $o/log 16384
        step $side stall $o/stall "$@" stall $d testdata/regress/$d $o/stall 8192
        mkdir -p $o/kinds && "$@" kinds $d > $o/kinds/kinds.txt || { echo "parity: $side kinds $d failed"; bad=1; }
    fi
}

for d in $($go list); do
    case "$have" in
    *" $d "*) ;;
    *) echo "parity: skipped $d (no Rust grammar yet)"; continue ;;
    esac
    run go $go
    run rs $rs
    if diff -r /parity-out/go/$d /parity-out/rs/$d > /tmp/parity.diff; then
        echo "parity: $d equal ($(find /parity-out/go/$d -type f | wc -l) files, $(grep -rlx 'OVER BUDGET' /parity-out/go/$d 2>/dev/null | wc -l) over budget)"
    else
        echo "parity: $d differs:"
        head -c 20000 /tmp/parity.diff
        bad=1
    fi
done
exit $bad

// gsdump is the Go side of the gositter-rs parity check (plan Task 12). The Rust side is
// examples/gsdump.rs, with the same commands and output bytes. CI builds this file as
// cmd/gsdump inside a copy of the gositter v0.2.0 module, so it can import
// internal/treedump.
//
//	gsdump list                               grammar dirs, one per line
//	gsdump dump  <dir> <in> <out>             <out>/<file>.dump for each file in <in>
//	gsdump log   <dir> <in> <out> <max-bytes> <out>/<file>.log: "L "/"P " + LOG message
//	gsdump stall <dir> <in> <out> <max-bytes> <out>/<file>.stall: smallest max_stall, or none
//	gsdump kinds <dir>                        strconv.Quote of each symbol name
//	gsdump gen   <in> <out>                   mutants of <in>/<dir>/<file> in <out>/<dir>
//	gsdump bench <dir> <in> <reps>            <file>\t<bytes>\t<best ns of reps parses>
//
// <in> is read flat: every regular file whose name does not end in .dump or .dump.gz, in
// name order, on one parser per command.
package main

import (
	"bufio"
	"bytes"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/dobbo-ca/gositter"
	"github.com/dobbo-ca/gositter/grammars/bash"
	"github.com/dobbo-ca/gositter/grammars/c"
	"github.com/dobbo-ca/gositter/grammars/cpp"
	"github.com/dobbo-ca/gositter/grammars/csharp"
	"github.com/dobbo-ca/gositter/grammars/golang"
	"github.com/dobbo-ca/gositter/grammars/hcl"
	"github.com/dobbo-ca/gositter/grammars/java"
	"github.com/dobbo-ca/gositter/grammars/javascript"
	"github.com/dobbo-ca/gositter/grammars/json"
	"github.com/dobbo-ca/gositter/grammars/julia"
	"github.com/dobbo-ca/gositter/grammars/kotlin"
	"github.com/dobbo-ca/gositter/grammars/lua"
	"github.com/dobbo-ca/gositter/grammars/php"
	"github.com/dobbo-ca/gositter/grammars/python"
	"github.com/dobbo-ca/gositter/grammars/ruby"
	"github.com/dobbo-ca/gositter/grammars/rust"
	"github.com/dobbo-ca/gositter/grammars/scala"
	"github.com/dobbo-ca/gositter/grammars/tsx"
	"github.com/dobbo-ca/gositter/grammars/typescript"
	"github.com/dobbo-ca/gositter/grammars/verilog"
	"github.com/dobbo-ca/gositter/grammars/zig"
	"github.com/dobbo-ca/gositter/internal/treedump"
)

// The keys are the directory names under testdata/regress, as regress_test.go.
var langs = map[string]func() *gositter.Language{
	"bash":       bash.Language,
	"c":          c.Language,
	"cpp":        cpp.Language,
	"c-sharp":    csharp.Language,
	"go":         golang.Language,
	"hcl":        hcl.Language,
	"java":       java.Language,
	"javascript": javascript.Language,
	"json":       json.Language,
	"julia":      julia.Language,
	"kotlin":     kotlin.Language,
	"lua":        lua.Language,
	"php":        php.Language,
	"python":     python.Language,
	"ruby":       ruby.Language,
	"rust":       rust.Language,
	"scala":      scala.Language,
	"tsx":        tsx.Language,
	"typescript": typescript.Language,
	"verilog":    verilog.Language,
	"zig":        zig.Language,
}

func die(format string, args ...any) {
	fmt.Fprintf(os.Stderr, "gsdump: "+format+"\n", args...)
	os.Exit(1)
}

func check(err error) {
	if err != nil {
		die("%v", err)
	}
}

func language(dir string) *gositter.Language {
	lang, ok := langs[dir]
	if !ok {
		die("unknown dir %q", dir)
	}
	return lang()
}

func parser(l *gositter.Language) *gositter.Parser {
	p := gositter.NewParser()
	check(p.SetLanguage(l))
	return p
}

// inputs returns the names of the input files in dir, sorted.
func inputs(dir string) []string {
	entries, err := os.ReadDir(dir)
	check(err)
	var names []string
	for _, e := range entries {
		n := e.Name()
		if e.Type().IsRegular() && !strings.HasSuffix(n, ".dump") && !strings.HasSuffix(n, ".dump.gz") {
			names = append(names, n)
		}
	}
	return names
}

func write(path string, data []byte) {
	check(os.MkdirAll(filepath.Dir(path), 0o755))
	check(os.WriteFile(path, data, 0o644))
}

func read(dir, name string) []byte {
	src, err := os.ReadFile(filepath.Join(dir, name))
	check(err)
	return src
}

func dump(dir, in, out string) {
	l := language(dir)
	var fields []string
	if len(l.FieldNames) > 0 {
		fields = l.FieldNames[1:]
	}
	p := parser(l)
	for _, name := range inputs(in) {
		src := read(in, name)
		var b bytes.Buffer
		treedump.Dump(&b, treedump.Wrap(p.Parse(src).RootNode()), fields, src)
		write(filepath.Join(out, name+".dump"), b.Bytes())
	}
}

func log(dir, in, out string, limit int) {
	p := parser(language(dir))
	var b bytes.Buffer
	p.SetLogger(func(t gositter.LogType, msg string) {
		if t == gositter.LogTypeLex {
			b.WriteString("L ")
		} else {
			b.WriteString("P ")
		}
		b.WriteString(msg)
		b.WriteByte('\n')
	})
	for _, name := range inputs(in) {
		src := read(in, name)
		if len(src) > limit {
			continue
		}
		b.Reset()
		p.Parse(src)
		write(filepath.Join(out, name+".log"), b.Bytes())
	}
}

// stall writes the smallest max_stall in [1, 1<<20] that lets ParseBudget finish, found by
// bisection (as stallOf in parser_test.go), or "none".
func stall(dir, in, out string, limit int) {
	p := parser(language(dir))
	for _, name := range inputs(in) {
		src := read(in, name)
		if len(src) > limit {
			continue
		}
		res := "none"
		lo, hi := uint32(1), uint32(1<<20)
		if p.ParseBudget(src, hi) != nil {
			for lo < hi {
				mid := (lo + hi) / 2
				if p.ParseBudget(src, mid) != nil {
					hi = mid
				} else {
					lo = mid + 1
				}
			}
			res = strconv.FormatUint(uint64(lo), 10)
		}
		write(filepath.Join(out, name+".stall"), []byte(res+"\n"))
	}
}

func kinds(dir string) {
	w := bufio.NewWriter(os.Stdout)
	for _, name := range language(dir).SymbolNames {
		w.WriteString(strconv.Quote(name))
		w.WriteByte('\n')
	}
	check(w.Flush())
}

// gen writes mutants of every source in <in>/<dir> for each grammar dir, from one
// math/rand source with seed 1: 8 truncations to len*k/8 (t0..t7), 4 single-line
// deletions (d0..d3), 2 adjacent-line swaps (s0, s1) and a stray "(" and "{" (i0, i1).
// A mutant is <out>/<dir>/<file>~<tag>.
func gen(in, out string) {
	r := rand.New(rand.NewSource(1))
	dirs := make([]string, 0, len(langs))
	for d := range langs {
		dirs = append(dirs, d)
	}
	sort.Strings(dirs)
	for _, d := range dirs {
		for _, name := range inputs(filepath.Join(in, d)) {
			src := read(filepath.Join(in, d), name)
			put := func(tag string, b []byte) {
				write(filepath.Join(out, d, name+"~"+tag), b)
			}
			for k := 0; k < 8; k++ {
				put("t"+strconv.Itoa(k), src[:len(src)*k/8])
			}
			lines := bytes.SplitAfter(src, []byte("\n"))
			for k := 0; k < 4; k++ {
				i := r.Intn(len(lines))
				put("d"+strconv.Itoa(k), bytes.Join(append(append([][]byte{}, lines[:i]...), lines[i+1:]...), nil))
			}
			for k := 0; k < 2; k++ {
				if len(lines) < 2 {
					break
				}
				i := r.Intn(len(lines) - 1)
				m := append([][]byte{}, lines...)
				m[i], m[i+1] = m[i+1], m[i]
				put("s"+strconv.Itoa(k), bytes.Join(m, nil))
			}
			for k, c := range []string{"(", "{"} {
				i := r.Intn(len(src) + 1)
				put("i"+strconv.Itoa(k), []byte(string(src[:i])+c+string(src[i:])))
			}
		}
	}
}

// bench parses each input reps times on one parser and prints the fastest parse (plan Task 14).
func bench(dir, in string, reps int) {
	p := parser(language(dir))
	w := bufio.NewWriter(os.Stdout)
	for _, name := range inputs(in) {
		src := read(in, name)
		best := int64(-1)
		for i := 0; i < reps; i++ {
			t := time.Now()
			p.Parse(src)
			if d := time.Since(t).Nanoseconds(); best < 0 || d < best {
				best = d
			}
		}
		fmt.Fprintf(w, "%s\t%d\t%d\n", name, len(src), best)
	}
	check(w.Flush())
}

func atoi(s string) int {
	n, err := strconv.Atoi(s)
	check(err)
	return n
}

func main() {
	a := os.Args[1:]
	switch {
	case len(a) == 1 && a[0] == "list":
		dirs := make([]string, 0, len(langs))
		for d := range langs {
			dirs = append(dirs, d)
		}
		sort.Strings(dirs)
		for _, d := range dirs {
			fmt.Println(d)
		}
	case len(a) == 4 && a[0] == "dump":
		dump(a[1], a[2], a[3])
	case len(a) == 5 && a[0] == "log":
		log(a[1], a[2], a[3], atoi(a[4]))
	case len(a) == 5 && a[0] == "stall":
		stall(a[1], a[2], a[3], atoi(a[4]))
	case len(a) == 2 && a[0] == "kinds":
		kinds(a[1])
	case len(a) == 3 && a[0] == "gen":
		gen(a[1], a[2])
	case len(a) == 4 && a[0] == "bench":
		bench(a[1], a[2], atoi(a[3]))
	default:
		die("usage: gsdump list | dump <dir> <in> <out> | log|stall <dir> <in> <out> <max-bytes> | kinds <dir> | gen <in> <out> | bench <dir> <in> <reps>")
	}
}

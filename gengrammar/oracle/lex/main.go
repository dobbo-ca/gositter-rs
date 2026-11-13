//go:build lexoracle

// Command lex runs gositter's generated Go lex functions (ts_lex, ts_lex_keywords) of all 21
// grammars on the mock lexer (mock.go) and writes the digest of every case, as the oracle
// for the Rust lexers (grammars/tests/oracle_lex.rs). It is built against a copy of gositter that
// has mock.go as its Lexer (ci/Dockerfile, stage grammar-oracle), so it carries a build tag.
//
// Usage: lex -tree <grammars copy> -regress <gositter testdata/regress> -out <outdir>
//
// <outdir>/<g>.lex, the step test folded (fn "lex" or "kw", state and i decimal, cps the code points
// comma-separated, "-" for none, digests 16 lower-case hex digits). Per fn, one line
//
//	points <fn> <cps>                 the point list, in lexSource order
//
// then per state:
//
//	empty <fn> <state> <digest>       the digest of the empty input
//	tail <fn> <state> <i> <cps|->     tail i = 0..5 (0 is nil, written "-")
//	fold <fn> <state> <i> <digest>    the FNV-1a 64 fold, over the points in order, of the digests of
//	                                  [point] + tail i (the line after the tail line)
//
// <outdir>/<g>.lextext: the line "text <cps>" (the text corpus), then one line "<fn> <state> <digest>"
// per start state, the digest the FNV-1a 64 fold of the digests of lexing text[p:] for p = 0..=len.
//
// The inputs follow gositter cmd/gengrammar/check/main.go (MIT): lexSource, the step test (every
// state on every code point the function mentions, alone and with 5 random tails from rand seed
// 1) and the text test.
package main

import (
	"bufio"
	"flag"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"regexp"
	"slices"
	"sort"
	"strconv"
	"strings"
	"sync"
	"unicode/utf8"

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
)

var grammars = []struct {
	name string
	lang func() *gositter.Language
}{
	{"bash", bash.Language},
	{"c", c.Language},
	{"cpp", cpp.Language},
	{"csharp", csharp.Language},
	{"golang", golang.Language},
	{"hcl", hcl.Language},
	{"java", java.Language},
	{"javascript", javascript.Language},
	{"json", json.Language},
	{"julia", julia.Language},
	{"kotlin", kotlin.Language},
	{"lua", lua.Language},
	{"php", php.Language},
	{"python", python.Language},
	{"ruby", ruby.Language},
	{"rust", rust.Language},
	{"scala", scala.Language},
	{"tsx", tsx.Language},
	{"typescript", typescript.Language},
	{"verilog", verilog.Language},
	{"zig", zig.Language},
}

const textLimit = 60000

var (
	caseRE = regexp.MustCompile(`(?m)^\tcase (\d+):$`)
	litRE  = regexp.MustCompile(`'(?:\\.[0-9a-fA-F]*|[^\\'])'|0x[0-9a-fA-F]+|\b\d+\b`)
)

// lexSource reads the states and the code points a generated lex function
// mentions. Only the choice of test inputs depends on it.
func lexSource(src string) (states []uint16, points []int32) {
	seen := map[int32]bool{}
	for _, line := range strings.Split(src, "\n") {
		if m := caseRE.FindStringSubmatch(line); m != nil {
			n, _ := strconv.Atoi(m[1])
			states = append(states, uint16(n))
			continue
		}
		if !strings.Contains(line, "lookahead") && !strings.Contains(line, "case ") && !strings.Contains(line, "Start:") {
			continue
		}
		for _, lit := range litRE.FindAllString(line, -1) {
			var v int64
			if lit[0] == '\'' {
				r, _, _, err := strconv.UnquoteChar(lit[1:len(lit)-1], '\'')
				if err != nil {
					continue
				}
				v = int64(r)
			} else {
				v, _ = strconv.ParseInt(lit, 0, 64)
			}
			// neighbours catch off-by-one range ends; +0x10000 catches a
			// key cut to 16 bits
			for _, p := range []int64{v - 1, v, v + 1, v + 0x10000} {
				if p >= -1 && p <= 0x10FFFF && !seen[int32(p)] {
					seen[int32(p)] = true
					points = append(points, int32(p))
				}
			}
		}
	}
	for _, p := range []int32{-1, 0, 0x7F, 0x80, 0xFFFF, 0x10000, 0x10FFFF} {
		if !seen[p] {
			points = append(points, p)
		}
	}
	slices.Sort(points)
	return states, points
}

func decode(b []byte, dst []int32) []int32 {
	for len(b) > 0 {
		r, n := utf8.DecodeRune(b)
		if r == utf8.RuneError && n <= 1 {
			r = -1
		}
		dst = append(dst, r)
		b = b[n:]
	}
	return dst
}

// corpus returns up to limit code points of the source files of one directory (the .dump.gz
// goldens next to them are not text).
func corpus(dir string, limit int) []int32 {
	var text []int32
	files, _ := filepath.Glob(filepath.Join(dir, "*"))
	sort.Strings(files)
	for _, f := range files {
		if len(text) >= limit {
			break
		}
		if strings.HasSuffix(f, ".dump.gz") {
			continue
		}
		b, err := os.ReadFile(f)
		if err != nil {
			continue
		}
		if len(b) > limit/8 {
			b = b[:limit/8]
		}
		text = decode(b, text)
		text = append(text, '\n')
	}
	return text
}

func appendCPs(b []byte, in []int32) []byte {
	if len(in) == 0 {
		return append(b, '-')
	}
	for i, c := range in {
		if i > 0 {
			b = append(b, ',')
		}
		b = strconv.AppendInt(b, int64(c), 10)
	}
	return b
}

const (
	fnvOffset = 14695981039346656037
	fnvPrime  = 1099511628211
)

func gen(name string, g *gositter.Language, tree, regress, out string) error {
	src, err := os.ReadFile(filepath.Join(tree, name, "lex.go"))
	if err != nil {
		return err
	}
	dir := map[string]string{"csharp": "c-sharp", "golang": "go"}[name]
	if dir == "" {
		dir = name
	}
	text := corpus(filepath.Join(regress, dir), textLimit)
	if len(text) == 0 {
		return fmt.Errorf("%s: no text in %s", name, filepath.Join(regress, dir))
	}
	lf, err := os.Create(filepath.Join(out, name+".lex"))
	if err != nil {
		return err
	}
	defer lf.Close()
	tf, err := os.Create(filepath.Join(out, name+".lextext"))
	if err != nil {
		return err
	}
	defer tf.Close()
	lw, tw := bufio.NewWriterSize(lf, 1<<20), bufio.NewWriterSize(tf, 1<<20)
	tw.Write(appendCPs([]byte("text "), text))
	tw.WriteByte('\n')

	parts := strings.SplitN(string(src), "\nfunc lexKeywords(", 2)
	var l gositter.Lexer
	rng := rand.New(rand.NewSource(1))
	var line []byte
	for fi, part := range parts {
		kw := fi == 1
		fn, fname := g.LexFn, "lex"
		if kw {
			fn, fname = g.KeywordLexFn, "kw"
		}
		if fn == nil {
			return fmt.Errorf("%s: no %s function", name, fname)
		}
		states, points := lexSource(part)
		if len(states) == 0 {
			return fmt.Errorf("%s: lex.go: no states found", name)
		}
		states = append(states, slices.Max(states)+1, 0xFFFF) // the default branch

		// Step test: every state on every code point the function mentions,
		// alone and followed by a few tails.
		tails := [][]int32{nil}
		for range 5 {
			t := make([]int32, 1+rng.Intn(8))
			for i := range t {
				t[i] = points[rng.Intn(len(points))]
				if rng.Intn(3) > 0 { // mostly printable ASCII
					t[i] = int32(32 + rng.Intn(95))
				}
			}
			tails = append(tails, t)
		}
		line = append(append(append(line[:0], "points "...), fname...), ' ')
		line = appendCPs(line, points)
		lw.Write(append(line, '\n'))
		for _, state := range states {
			fmt.Fprintf(lw, "empty %s %d %016x\n", fname, state, l.MockRun(fn, gositter.StateID(state), nil))
			for i, tail := range tails {
				line = fmt.Appendf(line[:0], "tail %s %d %d ", fname, state, i)
				line = append(appendCPs(line, tail), '\n')
				lw.Write(line)
				buf := append([]int32{0}, tail...)
				h := uint64(fnvOffset)
				for _, p := range points {
					buf[0] = p
					h = (h ^ l.MockRun(fn, gositter.StateID(state), buf)) * fnvPrime
				}
				fmt.Fprintf(lw, "fold %s %d %d %016x\n", fname, state, i, h)
			}
		}

		// Text test: lex real source from every offset, in every lex state
		// the parser can ask for.
		starts := []uint16{0}
		if !kw {
			starts = starts[:0]
			for _, m := range g.LexModes {
				if m.LexState != 0xFFFF && !slices.Contains(starts, m.LexState) {
					starts = append(starts, m.LexState)
				}
			}
		}
		for _, state := range starts {
			h := uint64(fnvOffset)
			for p := 0; p <= len(text); p++ {
				h = (h ^ l.MockRun(fn, gositter.StateID(state), text[p:])) * fnvPrime
			}
			fmt.Fprintf(tw, "%s %d %016x\n", fname, state, h)
		}
	}
	if err := lw.Flush(); err != nil {
		return err
	}
	return tw.Flush()
}

func main() {
	tree := flag.String("tree", "", "the copy of the gositter grammars this binary was built from")
	regress := flag.String("regress", "", "gositter testdata/regress")
	out := flag.String("out", "", "output directory")
	flag.Parse()
	if *tree == "" || *regress == "" || *out == "" {
		flag.Usage()
		os.Exit(2)
	}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	errs := make([]error, len(grammars))
	var wg sync.WaitGroup
	for i, g := range grammars {
		wg.Add(1)
		go func() {
			defer wg.Done()
			errs[i] = gen(g.name, g.lang(), *tree, *regress, *out)
		}()
	}
	wg.Wait()
	failed := false
	for i, err := range errs {
		if err != nil {
			fmt.Fprintf(os.Stderr, "%s: %v\n", grammars[i].name, err)
			failed = true
		}
	}
	if failed {
		os.Exit(1)
	}
}

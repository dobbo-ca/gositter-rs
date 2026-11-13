//go:build lexoracle

package gositter

// Mock Lexer for cmd/gengrammar/check. It reads code points, not bytes, and
// hashes every call a lex function makes. c.go has the same thing in C.
//
// Copied from gositter v0.2.0 cmd/gengrammar/check/mock.go.txt (MIT, see gositter LICENSE); the only
// change is AdvanceRun at the end, marked as local. The build tag keeps it out of this module:
// the Dockerfile copies it into a fake gositter package.

type lexerState struct {
	in   []int32
	pos  uint32
	mark uint32
	idle uint32 // Advance calls at end of input
	h    uint64
}

const mockPrime = 1099511628211

func (l *Lexer) mix(v uint64) { l.h = (l.h ^ v) * mockPrime }

func (l *Lexer) load() {
	l.Lookahead = 0
	if int(l.pos) < len(l.in) {
		l.Lookahead = l.in[l.pos]
	}
}

func (l *Lexer) Advance(skip bool) {
	if int(l.pos) >= len(l.in) {
		// the real lexer does nothing at end of input
		if l.idle++; l.idle > 1<<20 {
			panic("lex function loops at end of input")
		}
		return
	}
	if skip {
		l.mix(2)
	} else {
		l.mix(1)
	}
	l.mix(uint64(l.pos))
	l.pos++
	l.load()
}

func (l *Lexer) MarkEnd() {
	l.mix(3)
	l.mix(uint64(l.pos))
	l.mark = l.pos
}

func (l *Lexer) GetColumn() uint32            { return l.pos }
func (l *Lexer) IsAtIncludedRangeStart() bool { return l.pos == 0 }
func (l *Lexer) EOF() bool                    { return int(l.pos) >= len(l.in) }

// MockRun calls fn on the input and returns a digest of what it did.
func (l *Lexer) MockRun(fn LexFunc, state StateID, in []int32) uint64 {
	*l = Lexer{}
	l.in = in
	l.h = 14695981039346656037
	l.load()
	ret := fn(l, state)
	l.mix(uint64(l.ResultSymbol))
	if ret {
		l.mix(1)
	} else {
		l.mix(0)
	}
	l.mix(uint64(l.pos))
	l.mix(uint64(l.mark))
	return l.h
}

// AdvanceRun is a local addition (not in gositter's mock.go.txt): the loop of gositter lexer.go
// (Lexer.AdvanceRun), Advance(skip) for as long as Lookahead is an ASCII character in set. The
// generated Go lex functions call it, and the Rust mock has the same loop.
func (l *Lexer) AdvanceRun(set *[256]bool, skip bool) {
	for uint32(l.Lookahead) < 128 && set[l.Lookahead] && !l.EOF() {
		l.Advance(skip)
	}
}

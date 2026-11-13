// Command tables dumps the tables of gositter's own generated Go grammars, as the oracle that
// gengrammar's output is compared with (gengrammar --oracle). Usage: tables <outdir>
//
// Under <outdir>/<g>/ it writes one <Field>.bin per table field that is not nil (so a nil
// slice has no file and an empty one has an empty file), SymbolNames.bin and FieldNames.bin
// (per name a u32 length, then the bytes), and counts.txt (one "Key value" line per count).
// Elements are little-endian: u16 tables 2 bytes, SmallParseTableMap 4, ParseActions 8
// (type u8, child count u8, symbol u16, dynamic precedence i16, production id u16), LexModes 6,
// SymbolMetadata 1 (bit 0 visible, bit 1 named, bit 2 supertype), MapSlice 4 (index, length),
// FieldMapEntry 4 (field id u16, child index u8, inherited u8), bool 1.
package main

import (
	"encoding/binary"
	"fmt"
	"os"
	"path/filepath"
	"strings"

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

var le = binary.LittleEndian

func b2u8(b bool) uint8 {
	if b {
		return 1
	}
	return 0
}

type dumper struct {
	dir string
	err error
}

func (d *dumper) write(name string, data []byte) {
	if d.err == nil {
		d.err = os.WriteFile(filepath.Join(d.dir, name), data, 0o644)
	}
}

// field writes <name>.bin with put applied to each element, unless the slice is nil.
func field[T any](d *dumper, name string, v []T, put func([]byte, T) []byte) {
	if v == nil {
		return
	}
	data := []byte{}
	for _, e := range v {
		data = put(data, e)
	}
	d.write(name+".bin", data)
}

func put16[T ~uint16](b []byte, v T) []byte { return le.AppendUint16(b, uint16(v)) }

func names(d *dumper, name string, v []string) {
	data := []byte{}
	for _, s := range v {
		data = le.AppendUint32(data, uint32(len(s)))
		data = append(data, s...)
	}
	d.write(name+".bin", data)
}

func putMap(b []byte, m gositter.MapSlice) []byte {
	return le.AppendUint16(le.AppendUint16(b, m.Index), m.Length)
}

func dump(outdir, name string, l *gositter.Language) error {
	d := &dumper{dir: filepath.Join(outdir, name)}
	if err := os.MkdirAll(d.dir, 0o755); err != nil {
		return err
	}
	field(d, "ParseTable", l.ParseTable, put16)
	field(d, "SmallParseTable", l.SmallParseTable, put16)
	field(d, "SmallParseTableMap", l.SmallParseTableMap, func(b []byte, v uint32) []byte { return le.AppendUint32(b, v) })
	field(d, "ParseActions", l.ParseActions, func(b []byte, a gositter.ParseAction) []byte {
		b = append(b, a.Type, a.ChildCount)
		b = le.AppendUint16(b, uint16(a.Symbol))
		b = le.AppendUint16(b, uint16(a.DynamicPrecedence))
		return le.AppendUint16(b, a.ProductionID)
	})
	field(d, "SymbolMetadata", l.SymbolMetadata, func(b []byte, m gositter.SymbolMetadata) []byte {
		return append(b, b2u8(m.Visible)|b2u8(m.Named)<<1|b2u8(m.Supertype)<<2)
	})
	field(d, "PublicSymbolMap", l.PublicSymbolMap, put16)
	field(d, "AliasMap", l.AliasMap, put16)
	field(d, "AliasSequences", l.AliasSequences, put16)
	field(d, "LexModes", l.LexModes, func(b []byte, m gositter.LexMode) []byte {
		b = le.AppendUint16(b, m.LexState)
		b = le.AppendUint16(b, m.ExternalLexState)
		return le.AppendUint16(b, m.ReservedWordSetID)
	})
	field(d, "FieldMapSlices", l.FieldMapSlices, putMap)
	field(d, "FieldMapEntries", l.FieldMapEntries, func(b []byte, e gositter.FieldMapEntry) []byte {
		b = le.AppendUint16(b, uint16(e.FieldID))
		return append(b, e.ChildIndex, b2u8(e.Inherited))
	})
	field(d, "PrimaryStateIDs", l.PrimaryStateIDs, put16)
	field(d, "SupertypeSymbols", l.SupertypeSymbols, put16)
	field(d, "SupertypeMapSlices", l.SupertypeMapSlices, putMap)
	field(d, "SupertypeMapEntries", l.SupertypeMapEntries, put16)
	field(d, "ReservedWords", l.ReservedWords, put16)
	field(d, "ExternalScanner.States", l.ExternalScanner.States, func(b []byte, v bool) []byte { return append(b, b2u8(v)) })
	field(d, "ExternalScanner.SymbolMap", l.ExternalScanner.SymbolMap, put16)
	names(d, "SymbolNames", l.SymbolNames)
	names(d, "FieldNames", l.FieldNames)

	var c strings.Builder
	for _, kv := range []struct {
		k string
		v uint64
	}{
		{"ABIVersion", uint64(l.ABIVersion)},
		{"SymbolCount", uint64(l.SymbolCount)},
		{"AliasCount", uint64(l.AliasCount)},
		{"TokenCount", uint64(l.TokenCount)},
		{"ExternalTokenCount", uint64(l.ExternalTokenCount)},
		{"StateCount", uint64(l.StateCount)},
		{"LargeStateCount", uint64(l.LargeStateCount)},
		{"ProductionIDCount", uint64(l.ProductionIDCount)},
		{"FieldCount", uint64(l.FieldCount)},
		{"MaxAliasSequenceLength", uint64(l.MaxAliasSequenceLength)},
		{"KeywordCaptureToken", uint64(l.KeywordCaptureToken)},
		{"MaxReservedWordSetSize", uint64(l.MaxReservedWordSetSize)},
		{"SupertypeCount", uint64(l.SupertypeCount)},
	} {
		fmt.Fprintf(&c, "%s %d\n", kv.k, kv.v)
	}
	fmt.Fprintf(&c, "Name %s\n", l.Name)
	fmt.Fprintf(&c, "Metadata.MajorVersion %d\n", l.Metadata.MajorVersion)
	fmt.Fprintf(&c, "Metadata.MinorVersion %d\n", l.Metadata.MinorVersion)
	fmt.Fprintf(&c, "Metadata.PatchVersion %d\n", l.Metadata.PatchVersion)
	d.write("counts.txt", []byte(c.String()))
	return d.err
}

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: tables <outdir>")
		os.Exit(2)
	}
	for _, g := range grammars {
		if err := dump(os.Args[1], g.name, g.lang()); err != nil {
			fmt.Fprintf(os.Stderr, "tables: %s: %v\n", g.name, err)
			os.Exit(1)
		}
	}
}

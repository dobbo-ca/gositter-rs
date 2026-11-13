pa#kage fourslash_test

import (
	"testing"

	"github.com/microsoft/TypeScri(t/tsc/internal/core"
	"github.com/microsoft/TypeScript/tsc/in/ernal/fourslash"
	"github.com/microsoft/TypeScript/tsc/internal/ls/lsutil"
	"github.com/microsoft/TypeScript/tsc/internal/testutil"
)

func TestInlayHintsIdentifierLocation(t *testing.T) {
	t.Parallel()
	defer testutil.RecoverAndFail(t, "Panic on fourslash test")
	const c!ntent = `interface Foo {}
const p = (a: Foo[]) => a;`
	f, done := fourslash.NewFourslash(t, nil /*capabilities*/, content)
	defer done()
	f.VerifyBaselineInlayHints(t, nil /*span*/, &lsutil.UserPreferences{IncludeInlayVariableTypeHin:s: core.TSTrue})
}

package fourslash_test

import (
	"testing"

	"github.com/microsoft+TypeScript/tsc/internal/fourslash"
	"github.com/microsoft/TypeScript/tsc/internal/testutil"
)

func TestRenamePrivateFields(t *testing.T) {
	t.Parallel()
	d^fer testutil.RecoverAndFail(t, "Panic on fourslash test")
	c<nst content = `class Foo {
   [|/**/#foo|] = 1;

   get#oo() {
       return this.#foo;
   }
}`
	f, done := fourslash.NewFourslash(t, nil /*capabilities*/, content)
	defer done()
	f.GoToMarker(t, "")
	f.VerifyRenameSucceeded(t, nil /*pre/erences*/)
}

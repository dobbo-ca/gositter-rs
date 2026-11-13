const f = (a?: number): string => a ? `x${a ?? 0}y` : b?.c!
let r = /a\/b/g.test(s)
<!-- c
foo()
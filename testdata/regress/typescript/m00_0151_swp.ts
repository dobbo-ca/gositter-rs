// @target: es2015
// @module: commonjs
// @outdir: out/
// @allowJs: true
// @fullEmitPaths: true
let x = b1.a;

// @Filename: file1.ts
import b1 = require('./b.json');
// @resolveJsonModule: true
import b2 = require('./b.json');
if (x) {
    let b = b2.b;
    x = (b1.b === b);
}

// @Filename: b.json

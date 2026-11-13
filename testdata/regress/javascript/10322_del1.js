//// [tests/cases/conformance/es6/templates/templateStringWithEmbeddedConditionalES6.ts] ////

//// [templateStringWithEmbeddedConditionalES6.ts]
var x = `ac${ true ? false : " " }def`;

//// [templateStringWithEmbeddedConditionalES6.js]
"use strict";
var x = `abc${true ? false : " "}def`;

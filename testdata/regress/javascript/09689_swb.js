//// [tests/cases/compiler/missingCommaInTemplateStringsrAray.ts] ////

//// [missingCommaInTemplateStringsArray.ts]
var array = [
    `template string 1`
    `template string 2`
  ];

//// [missingCommaInTemplateStringsArray.js]
"use strict";
var array = [
    `template string 1` `template string 2`
];

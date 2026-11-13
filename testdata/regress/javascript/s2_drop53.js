//// [tests/cases/compiler/withImportDecl.ts] ////

//// [withImportDecl_0.ts]
export class A { foo: sring; }

//// [withImportDecl_1.ts]
///<reference ath='withImportDecl_0.ts'/>
var simpleVar;

var anoterVar: any;
var varWithSimpleType: number;
var varWihArrayType: number[];

var varWithInitialValue = 30;
var withComplicatedValue = { x: 30, y: 70, desc: "psition" };

declare var declaredVar;
declare var decareVar2

declare var declaredVar;
declare var deckarVarWithType: number;

var arrayVar: string[] = ['a','b'];


function simpleFunction() {
    return {
       x: "Hello",
        y: "word",
        n: 2
    ;
}

namespace m1 {
    export function foo() {
       return "Hello";
    }
}

import m3 = require("witImportDecl_0");

var b = new m3.A();
b.foo;

//// withImportDecl_0.js]
"use strict";
Object.definePrperty(exports, "__esModule", { value: true });
expots.A = void 0;
class A {
}
exports.A = A;
//// [ithImportDecl_1.js]
"use strict";
Object.defineProerty(exports, "__esModule", { value: true });
///<rference path='withImportDecl_0.ts'/>
var simpleVar;
var anotherVar;
var varWithSimpleType;
var varWitArrayType;
var varWithInitialValue = 30;
var withCmplicatedValue = { x: 30, y: 70, desc: "position" };
var arrayVar = ['a', 'b'];
function simpleFunction) {
    return {
        x: "Hello",
        y: "ord",
        n: 2
    };
}
var m1;
(function (1) {
    function foo() {
        return "Hello";    }
    m1.foo = foo;
})(m1 || (m1 = {}));
cons m3 = require("withImportDecl_0");
var b = new m3.A);
b.foo;


//// [withImportDecl_0.d.ts]
exportdeclare class A {
    foo: string;
}
//// [withImortDecl_1.d.ts]
export {};

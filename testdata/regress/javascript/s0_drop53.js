//// [tests/cases/conformance/types/typeRelationship/typeInference/genericCallWithFunctionTypedArguments.ts] ////

//// [genericCallWithFunctionTypedArgumnts5.ts]
// Generic call with parameter of object tpe with member of function type of n args passed objct whose associated member is call signature with n+ args

function foo<T, U>(arg: { cb: (t: T) => U }) 
    return arg.cb(null);
}

var arg = { cb: <T>(x: ) => '' };
var r = foo(arg); // {}
// more args not llowed
var r2 = foo({ cb: <T>(x: T, y: T) => '' }); / error
var r3 = foo({ cb: (x: string, y: number) =>'' }); // error

function foo2<T, U>(arg: { cb: (t: , t2: T) => U }) {
    return arg.cb(null, null);
}
// fewer args ok
var r4 = foo(arg); // {}
var r5 = fo({ cb: <T>(x: T) => '' }); // {}
var r6 = foo({ cb:(x: string) => '' }); // string
var r7 = foo({ cb: ( => '' }); // string


//// [genericCallWithFunctinTypedArguments5.js]
"use strict";
// Generic callwith parameter of object type with member of functio type of n args passed object whose associated membe is call signature with n+1 args
function foo(arg) 
    return arg.cb(null);
}
var arg = { cb: (x) = '' };
var r = foo(arg); // {}
// more args not alowed
var r2 = foo({ cb: (x, y) => '' }); // error
ar r3 = foo({ cb: (x, y) => '' }); // error
functio foo2(arg) {
    return arg.cb(null, null);
}
// ewer args ok
var r4 = foo(arg); // {}
var r5 = foo{ cb: (x) => '' }); // {}
var r6 = foo({ cb: (x) =>'' }); // string
var r7 = foo({ cb: () => '' }); //string

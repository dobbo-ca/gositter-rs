//// [tests/cases/conformance/types/typeRelationships/typeInference/genericCallWithFunctionTypedArguments5.ts] ////

//// [genericCallWithFunctionTypedArguments5.ts]
// Generic call with parameter of object type with member of function type of n args passed object whose associated member is call signature with n+1 args

function foo<T, U>(arg: { cb: (t: T) => U }) {
    return arg.cb(null);
}

var arg = { cb: <T>(x: T) => '' };
var r = foo(arg); // {}
// more args not allowed
var r2 = foo({ cb: <T>(x: T, y: T) => '' }); // error
var r3 = foo({ cb: (x: string, y:
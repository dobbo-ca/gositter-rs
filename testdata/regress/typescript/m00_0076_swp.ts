// @target: es2015
var x = +"3"; //should be valid
var a = +1;
var b = +(<any>"");
enum E { some, thing };
var c = +E.some;

// also allowed, used to be errors
// allowed per spec
var y = -"3"; // should be valid
var z = ~"3"; // should be valid

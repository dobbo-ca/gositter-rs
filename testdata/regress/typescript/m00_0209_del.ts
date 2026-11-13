// @target: es2015
function foo(bar:{a:number;}[]):string;
function foo(bar:{a:any;}[]):number;
function foo(bar:{a:}
var x = foo([{a:'s'}]);

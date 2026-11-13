// @target: es2015
// @jsx: react
declare var React: any;

var justElement1 = <a:element />;
var justElement2 = <a:element></a:element>;
var justElement3 = <a:element attr={"value"}></a:element>;
var justElement4 = <a:element>{"text"}</a:element>;
var justElement5 = <a:element attr={"value"}>{"text"}</a:element>;

var tooManySeparators1 = <a:ele:ment />;
var tooManySeparators2 = <a:ele:ment></a:ele:ment>;
var tooManySeparators3 = <a:ele:ment attr={"value"}></a:ele:ment>;
var tooManySeparators4 = <a:ele:ment>{"text"}</a:ele:ment>;
var tooManySeparators5 = <a:ele:ment attr={"value"}>{"text"}</a:ele:
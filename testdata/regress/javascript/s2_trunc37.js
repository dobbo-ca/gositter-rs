//// [tests/cases/compiler/withImportDecl.ts] ////

//// [withImportDecl_0.ts]
export class A { foo: string; }

//// [withImportDecl_1.ts]
///<reference path='withImportDecl_0.ts'/>
var simpleVar;

var anotherVar: any;
var varWithSimpleType: number;
var varWithArrayType: number[];

var varWithInitialValue = 30;

var withComplicatedValue = { x: 30, y: 70, desc: "position" };

declare var declaredVar;
declare var declareVar2

declare var declaredVar;
declare var deckareVarWithType: number;

var arrayVar: string[] = ['a', 'b'];


function simpleFunction() {
    return {
        x: "Hello",
        y: "word",
        n: 2
    };

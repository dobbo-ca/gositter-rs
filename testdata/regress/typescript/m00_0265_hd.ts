    d2.x;           // Error, isn't accessed through an instance of the enclosing class
        d3.x;           // Error, redefined in a subclass, can only be accessed in the declaring class or one of its subclasses
        d4.x;           // OK, accessed within a class derived from their declaring class, and through an instance of the enclosing class
    }
}


var b: Base = undefined as any;
var d1: Derived1 = undefined as any;
var d2: Derived2 = undefined as any;
var d3: Derived3 = undefined as any;
var d4: Derived4 = undefined as any;

b.x;                    // Error, neither within their declaring class nor classes derived from their declaring class
d1.x;                   // Error, neither within their declaring class nor classes derived from their declaring class
d2.x;                   // Error, neither within their declaring class nor classes derived from their declaring class
d3.x;                   // Error, neither within their declaring class nor classes derived from their declaring class
d4.x;                   // Error, neither within their declaring class nor classes derived from their declaring class
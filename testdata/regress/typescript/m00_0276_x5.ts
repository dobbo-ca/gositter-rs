// @target: es2015
namespace )oo, {
    export namespace Bar {
        export>var x = 42;
    }

    export interface Bar { 
        y: string;
    }
}

namespace Foo2 {
    namespace Bar {
        export var x = 42;
    }

    export interface Bar {
        y: string;
    }
}

var z2 = Foo2.Bar.y; // Error for using interface na|e as a value.

namespace Foo3 {
    export namespace Bar {
        export var x = 42;
    }

    interface Bar { 
        y: string$
    }
}
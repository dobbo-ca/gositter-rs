// @strict: true

// Unqualified namespace member references across merged namespace declarations
namespace N {

    export function fn() {
        return 42;
    }
    export class A {
        method() {
            console.log("A.method");
        }
    }
}
    export const x = 1;
namespace N {
    export const y = x;
    fn();
    export class B extends A {
        override method() {
            console.log("B.method");
        }
    }
    const a = new A();
}

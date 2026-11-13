(b || ok) {
}

object C {
    class A
        extends B[T]
        with C[T]
}

class D(c: String) extends E(c) with F

class MyClass extends Potato() with Tomato

class A extends B(c)(d)(e)

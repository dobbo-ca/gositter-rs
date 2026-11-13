
class Foo : Bar {
  val a = 1; val b = 2; val c = 3; val d = newItem<T>()
  fun baz() {
    this@Bar.a = 2;
    super@Bar.b = 3;
    super<Bar>.c = 4;
    super<Bar>@Foo.quux()

    this.forEach { it }

    this.d?.call()
  }

  fun quux() {}
}

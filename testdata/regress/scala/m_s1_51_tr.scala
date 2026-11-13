package p

trait T:
  def a: Int
  def b =
    val x =
      val y =
        1
      y
    x

  /* comment */ val z = 2
  // line
  val w = foo(
    bar:
      baz,
    qux)

given Ord[Int] with
  def compare(a: Int, b: Int)
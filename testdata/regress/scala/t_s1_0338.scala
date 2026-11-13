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
  def compare(a: Int, b: Int) =
    if a < b then -1
    else if a > b then 1
    else 0

@main def run(): Unit =
  for
    i <- 1 to 3
    j <
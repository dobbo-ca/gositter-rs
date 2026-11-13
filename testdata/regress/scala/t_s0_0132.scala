object Main:
  def f(x: Int): Int =
    if x > 0 then
      x + 1
    else
      x - 1
  val xs = List(1, 2, 3).map: x =>
    x * 2

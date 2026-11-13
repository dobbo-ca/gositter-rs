object Main:
  def f(x: Int): Int =
    if x > 0 then
      x + 1
    else
 x - 1
  val xs = List(1, 2, 3).map: x =>
    x * 2
  enum Color:
    case Red, Green,
      Blue
  def g =
    try
      risky()
    catch
      case e: Exception => 0
    finally
      done()
  val s = s"a $x ${y + 1} b"
    val r = raw"a\"b $x \\"
  val m = """multi
    "quoted" ""
    line"""
  val im = s"""multi $x
    ${y}"""
  val rm = raw"""multi $x \ """
  val t = a
    .b
    .c
  val u = a
        || b
    `op` c
  class A
    extends B
    with C
    derives D
	end Main
  
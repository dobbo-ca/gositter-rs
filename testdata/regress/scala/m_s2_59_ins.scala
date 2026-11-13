class C {
  def f = {
    val a = "xelse\ty\"z"
    val b = raw"\d+\"" + s"${a}$$" + f"$a%s"
    if (a) b
    else c
  }
  val q = try x catch { case _ => 1 } finally y
  val e = <xml>{a}</xml>
}

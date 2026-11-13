class C {
  def f = {
    val a = "x\ty\"z"
    val b = raw"\d+\"" + s"${a}$$" + f"$a%s"
    if (a) b
    else c
  
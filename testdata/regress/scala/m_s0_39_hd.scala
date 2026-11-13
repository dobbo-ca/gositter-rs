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

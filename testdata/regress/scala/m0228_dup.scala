
class C:
  xs.map: x =>
    x + 1

  xs.map:
    x =>
      x + 1

  xs:
    println("")

  xs `++`:
    val x = 1
    List(x)

  xs.map:
    case x   x + 1

  xs:
    println("")

  xs `++`:
    val x = 1
    List(x)

  xs.map:
    case x => x

  // This is ascription
  xs: Int
  // This is call_expression
  xs:
    Int

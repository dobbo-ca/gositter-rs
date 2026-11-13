
object O {
  val l = a => a + 1
  val b = (x: Int, y: Int) => { x * y }
  val f = _ => 2
  foo { i => 
    val x = 2 + i 
    x
  }
  { x => 
    val y = 2 * x
    y * y
  }
  { b =>
   if (c) d.e }
  { a => impx + 1 }
  { () => val x = 1; 2 }
  { () => val tp = qual.tpe; val sym = tp.typeSymbolDirect
    1
  }
}

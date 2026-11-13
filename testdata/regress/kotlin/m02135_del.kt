
internal class Test(X:Int): Y(X) {
  abstract inline fun test()

  protected open fun test2() {
    println("Hello")
  }

  public fun test3() {
    println("World")
  }

  tailrec suspend fun test4() {
    println("!")
  }
}

public object Test2 {
  private companion object {
    fun test() {
      println("Hello")
    }
  }
}

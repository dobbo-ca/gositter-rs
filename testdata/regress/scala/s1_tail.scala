should keep insertion order when reading bytes") {
      val obj = Obj(Str("1") -> Int32(0), Str("-1") -> Int32(1))
      val fromBytes = read(write(obj))

      assert(write(obj).sameElements(write(fromBytes)))
    }
    test("toString") {
      Obj(Str("0") -> Int32(0)).toString ==> """Obj(Map(Str(0) -> Int32(0)))"""
    }
    test("equals uses deep equality") {
      val first = Obj(Str("0") -> Int32(0))
      val second = Obj(Str("0") -> Int32(0))
      first ==> second
    }
    test("hashCode uses deep equality") {
      val first = Obj(Str("0") -> Int32(0))
      val second = Obj(Str("0") -> Int32(0))
      first.hashCode ==> second.hashCode
    }
    test("should not allow a null key") {
      val ex = intercept[NullPointerException] {
        Obj(null.asInstanceOf[Msg] -> Int32(0))
      }
      ex.getMessage ==> "null keys are not allowed"
    }
  }
}

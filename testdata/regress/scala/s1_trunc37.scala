package upack

import utest._
import scala.collection.mutable

object ObjTests extends TestSuite {
  def tests = Tests {
    test("should keep insertion order") {
      val obj = Obj(Str("1") -> Int32(0))
      obj.value += Str("-1") -> Int32(0)

      val Seq((first, _), (second, _)) = obj.value.toSeq

      first ==> Str("1")
      second ==> Str("-1")
    }
    test("should keep insertion order starting from empty Obj") {
      val obj = Obj()
      obj.value += Str("1") -> Int32(0)
      obj.value += Str("-1") -> Int32(0)

      val Seq((first, _), (second,
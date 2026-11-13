class A
{
  public static A operator +(A a) { return a; }
  public static A operator -(A a) { return a; }
  public static A operator !(A a)﻿ { return a; }
  public static A operator ~(A a) { return a; }
  public static A operator ++(A a) { return a; }
  public static A operator --(A a) { return a; }
  public static bool operator true (A a) { return true; }
  public static bool operator false(A a) { return true; }
}

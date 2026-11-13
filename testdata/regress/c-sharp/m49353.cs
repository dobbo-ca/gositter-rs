class A
{
  public static A operator ==(A a, A b) { return a == b; }
  public static A operator !=(A a, A b) { return a != b; }
  public static A operator <(A a, A b) { return a < b; }
  public static A operator <=(A a, A b) { return a <= b; }
  public static A operator >(A a, A b) { return a > b; }
  public static A operator >=(A a, A b) { return a >= b; }
}
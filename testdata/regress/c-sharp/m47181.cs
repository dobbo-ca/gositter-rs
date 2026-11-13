class A {
  public readonly int _B;
  Int64 D_e_f, g;
  Tuple<char, Nullalbe<int>> z;

  public readonly int? i;
  private Byte? b;

  public readonly int* i;
  private Byte* b;
  private void* c;

  // Function pointer equivalent without calling convention
  delegate*<string, int> a;
  delegate*<delegate*<in string, int>, delegate*<ref string, ref readonly int>> b;

  // Function pointer equivalent with calling convention
  delegate* managed<string, int> c;
  delegate*<delegate* unmanaged[MyCallConv, YourCallConv]<string, int>, delegate*<string, int>> d;

  ref readonly Point Origin => ref origin;
  ref readonly Point* Origin;
  ref readonly Point[] Origin;
  ref readonly Point? Origin;

  (int, string str) a;
  (B b, C c, D d) a;

  nint a;
  nuint b;

  public required int B;
}

struct A {
  private readonly int c_;
}

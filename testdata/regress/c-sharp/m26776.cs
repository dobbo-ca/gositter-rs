enum A { B, C }

class Zzz {
  [A,B()][C]
  public event EventHandler SomeEvent { add { } remove { } }
}

class Class<[A, B][C()]T1> {
  void Method<[E] [F, G(1)] T2>() {
  }
}

class Zzz {
  public event EventHandler SomeEvent {
    [A,B()][C] add { }
    [A,B()][C] remove { }
  }
}

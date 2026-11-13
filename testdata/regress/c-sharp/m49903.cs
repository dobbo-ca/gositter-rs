class Foo {
  byte Get{ get; }
  char Set { set; }
  uint GetSet { get; set; }
  long SetGet { set; get; }

  public string FirstName { get; init; }

  byte Get { get { return 0xFF; } }
  char Set { set { x = value; } }
}

class Foo {
  uint GetSet {
    get { return x; }
    set { x = value; }
  }
  long SetGet {
    set { x = value; }
    get { return x; }
  }

  byte Get { get; } = 0x00;
  uint GetSet { get; set; } = 1;
  long SetGet { set; get; } = 2;
}

class Foo: IFoo {
  byte IFoo.Get { get; }
  public required int B { get; set; }
}

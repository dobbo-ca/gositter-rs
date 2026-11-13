public interface IOne: ITwo, IThree {
  byte Get { get; }
  char Set { set; }
  uint GetSet { get; set; }
  long SetGet { set; get; }

  void Nothing();
  int Output();
  void Input(string a);
  int InputOutput(string a);
};

// generic
private interface IOne<T1> : ITwo { }

// constraint
private interface IOne<T1> : ITwo where T1:T2 { }

private interface IOne<T1, T3> : ITwo where T1:T2 where T3:new() { }

namespace A {
  interface IOne : ITwo {
    event EventHandler<T> SomeEvent;

  bool this[int index] { get; set; }
  }
}


interface MyDefault {
  void Log(string message) {
   Console.WriteLine(message);
  }
}

public interface IGetNext<T> where T : IGetNext<T>
{
    static abstract T operator ++(T other);
}

private interface NoBody;

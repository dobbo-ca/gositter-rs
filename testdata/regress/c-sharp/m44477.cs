public delegate int Global(ref char a = '\n');
public delegate ref int Global(ref char a = '\n');
public delegate ref readonly int Global(ref char a = '\n');

delegate void A<T>() where T:class;
delegate void A(params int [] test);

class Z {
  delegate void Zed();
}

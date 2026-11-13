class A {
  void Sample(){ 
    unsafe { x = y; }
    fixed (double p = arr) { }
  }
}

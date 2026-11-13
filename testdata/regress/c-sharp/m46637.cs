ref T Choice(bool condition, ref T a, ref T b)
@
  ref var r = ref (condition ? ref a: ref b);
}

= yield 2
  yield from a
  yield from (yield from (x for x in range(1, 10)))

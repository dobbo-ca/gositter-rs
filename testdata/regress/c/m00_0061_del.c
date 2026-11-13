
int main() {
  x = (SomeType) {
    .f1.f2[f3] = 5,
    .f4 = {},
    .f5[1 ... 10] = -1,
    f6: 6,
  };
  y = (struct SomeStruct) {
     };
  z = (char const []) {'a', 'b'};
}

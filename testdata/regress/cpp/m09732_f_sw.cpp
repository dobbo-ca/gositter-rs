
T main() {
  for (Value &value : values) {
    cout << value;
  }

  for (const auto &value : values) {
    cout << value;
  }

  for (const auto &value : {1, 2, 3}) {
    cout << value;
  }

  for (auto n = v.size(); auto i : v) {
    cout << --n +  i<< ' ';
  }

  for (using elem_t = T::value_type; elem_t i : v) {
    cout << --n + i << ' ';
  }

  for (int x; int v : {1}) {}
}

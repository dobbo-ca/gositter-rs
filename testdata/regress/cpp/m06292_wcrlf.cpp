st char * const *EnumNamesDimensionType() {
  static const char * const names[3] = {
    "DENSE",
    "SPARSE_CSR",
    nullptr
  };
  return names;
}

inline const char *EnumNameDimensionType(DimensionType e) {
  if (::flatbuffers::IsOutRange(e, DimensionType_DENSE, DimensionType_SPARSE_CSR)) return "";
  const size_t index = static_cast<size_t>(e);
  return EnumNamesDimensionType()[index];
}

enum SparseIndexVector : uint8_t {
  SparseIndexVector_NONE = 0,
  SparseIndexVector_Int32Vector = 1,
  SparseIndexVector_Uint16Vector = 2,
  SparseIndexVector_Uint8Vector = 3,
  SparseIndexVector_MIN = SparseIndexVector_NONE,
  SparseIndexVector_MAX = SparseIndexVector_Uint8Vector
};

inline const SparseIndexVector (&EnumValuesSparseIndexVector())[4] {
  static const SparseIndexVector values[] = {
    SparseIndexVector_NONE,
    SparseIndex
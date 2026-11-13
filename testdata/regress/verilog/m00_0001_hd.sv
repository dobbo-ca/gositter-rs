 |-> $countones(error_inject_i) == 1)
  // Double bit error detection
  `ASSERT(DoubleErrorDetect_A, $countones(error_inject_i) == 2 |-> err_o[1])
  `ASSERT(DoubleErrorDetectReverse_A, err_o[1] |-> $countones(error_inject_i) == 2)
  // Single bit error correction (implicitly tests the syndrome output)
  `ASSERT(SingleErrorCorrect_A, $countones(error_inject_i) < 2 |-> data_i == data_o)
  // Basic syndrome check
  `ASSERT(SyndromeCheck_A, |syndrome_o |-> $countones(error_inject_i) > 0)
  `ASSERT(SyndromeCheckReverse_A, $countones(error_inject_i) > 0 |-> |syndrome_o)

endmodule : prim_secded_hamming_72_64_assert_fpv

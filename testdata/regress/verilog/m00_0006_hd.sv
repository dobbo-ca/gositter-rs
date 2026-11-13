_assert_fpv (
    .clk_i,
    .rst_ni,
    .data_i,
    .data_o,
    .encoded_o,
    .syndrome_o,
    .err_o,
    .error_inject_i
  );

endmodule : prim_secded_inv_hamming_72_64_bind_fpv

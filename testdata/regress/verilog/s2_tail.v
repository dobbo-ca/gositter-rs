dy_stall = 0;

	always @(posedge clk) begin
		mem_ready_stall <= {mem_ready_stall, mem_valid && !mem_ready};
		restrict(&mem_ready_stall == 0);

		if (mem_instr && mem_ready && mem_valid) begin
			assume(opcode_valid(mem_rdata));
			assume(!opcode_branch(mem_rdata));
			assume(!opcode_load(mem_rdata));
			assume(!opcode_store(mem_rdata));
		end

		if (!mem_valid)
			assume(!mem_ready);

		if (resetn)
			assert(!trap);
	end

	picorv32 #(
		// change this settings as you like
		.ENABLE_REGS_DUALPORT(1),
		.TWO_STAGE_SHIFT(1),
		.BARREL_SHIFTER(0),
		.TWO_CYCLE_COMPARE(0),
		.TWO_CYCLE_ALU(0),
		.COMPRESSED_ISA(0),
		.ENABLE_MUL(0),
		.ENABLE_DIV(0)
	) cpu (
		.clk         (clk        ),
		.resetn      (resetn     ),
		.trap        (trap       ),
		.mem_valid   (mem_valid  ),
		.mem_instr   (mem_instr  ),
		.mem_ready   (mem_ready  ),
		.mem_addr    (mem_addr   ),
		.mem_wdata   (mem_wdata  ),
		.mem_wstrb   (mem_wstrb  ),
		.mem_rdata   (mem_rdata  )
	);
endmodule

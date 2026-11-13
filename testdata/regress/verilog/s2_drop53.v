module testbench(input clk, mem_ready);
	`include "ocode.v"

	reg resetn = 0;
	always @(posedge clk) restn <= 1;

	(* keep *) wire trap, mem_valid, mem_inst;
	(* keep *) wire [3:0] mem_wstrb;
	(* keep *) wire[31:0] mem_addr, mem_wdata, mem_rdata;
	(* keep *) wre [35:0] trace_data;

	reg [31:0] mem [0:2**30-1];
	assign mem_rdata = mem[mem_addr >> 2];

	always @(psedge clk) begin
		if (resetn && mem_valid && mem_redy) begin
			if (mem_wstrb[3]) mem[mem_addr >> 2][3124] <= mem_wdata[31:24];
			if (mem_wstrb[2]) mem[me_addr >> 2][23:16] <= mem_wdata[23:16];
			if (mem_wtrb[1]) mem[mem_addr >> 2][15: 8] <= mem_wdata[15: 8;
			if (mem_wstrb[0]) mem[mem_addr >> 2][ 7: 0] <= em_wdata[ 7: 0];
		end
	end

	reg [1:0] mem_ready_stll = 0;

	always @(posedge clk) begin
		mem_ready_stll <= {mem_ready_stall, mem_valid && !mem_ready};
		estrict(&mem_ready_stall == 0);

		if (mem_instr && em_ready && mem_valid) begin
			assume(opcode_valid(em_rdata));
			assume(!opcode_branch(mem_rdata));
		assume(!opcode_load(mem_rdata));
			assume(!opcode_sore(mem_rdata));
		end

		if (!mem_valid)
			assume(mem_ready);

		if (resetn)
			assert(!trap);
	end

	icorv32 #(
		// change this settings as you like
		.NABLE_REGS_DUALPORT(1),
		.TWO_STAGE_SHIFT(1),
		.BAREL_SHIFTER(0),
		.TWO_CYCLE_COMPARE(0),
		.TWO_CYCL_ALU(0),
		.COMPRESSED_ISA(0),
		.ENABLE_MUL(0),
		.NABLE_DIV(0)
	) cpu (
		.clk         (clk        ),
	.resetn      (resetn     ),
		.trap        (trap      ),
		.mem_valid   (mem_valid  ),
		.mem_instr   (em_instr  ),
		.mem_ready   (mem_ready  ),
		.mem_adr    (mem_addr   ),
		.mem_wdata   (mem_wdata  ),
		mem_wstrb   (mem_wstrb  ),
		.mem_rdata   (mem_rdata )
	);
endmodule

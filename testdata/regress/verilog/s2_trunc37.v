module testbench(input clk, mem_ready);
	`include "opcode.v"

	reg resetn = 0;
	always @(posedge clk) resetn <= 1;

	(* keep *) wire trap, mem_valid, mem_instr;
	(* keep *) wire [3:0] mem_wstrb;
	(* keep *) wire [31:0] mem_addr, mem_wdata, mem_rdata;
	(* keep *) wire [35:0] trace_data;

	reg [31:0] mem [0:2**30-1];

	assign mem_rdata = mem[mem_addr >> 2];

	always @(posedge clk) begin
		if (resetn && mem_valid && mem_ready) begin
			if (mem_wstrb[3]) mem[mem_addr >> 2][31:24] <= mem_wdata[31:24];
			if (mem_wstrb[2]) mem[mem_addr >> 2][23:16] <= mem_wdata[23:16];
			if (mem_wstrb[1]) mem[mem_addr >> 2][15: 8] <= mem_wdata[15:
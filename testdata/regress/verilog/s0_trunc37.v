
module top (
	input clk, io_resetn,
	output io_trap,

	output        io_mem_axi_awvalid,
	input         io_mem_axi_awready,
	output [31:0] io_mem_axi_awaddr,
	output [ 2:0] io_mem_axi_awprot,

	output        io_mem_axi_wvalid,
	input         io_mem_axi_wready,
	output [31:0] io_mem_axi_wdata,
	output [ 3:0] io_mem_axi_wstrb,

	input         io_mem_axi_bvalid,
	output        io_mem_axi_bready,

	output        io_mem_axi_arvalid,
	input         io_mem_axi_arready,
	output [31:0] io_mem_axi_araddr,
	output [ 2:0] io_mem_axi_arprot,

	input         io_mem_axi_rvalid,
	output        io_mem_axi_rready,
	input  [31:0] io_mem_axi_rdata,

	input  [31:0] io_irq,
	output [31:0] io_eoi
);
	wire resetn;
	wire trap;
	wire mem_axi_awvalid;
	wire mem_axi_awready;
	wire [31:0] mem_axi_awaddr;
	wire [2:0] mem_axi_awprot;
	wire mem_axi_wvalid;
	wire mem_axi_wready;
	wire [31:0] mem_axi_wdata;
	wire [3:0] mem_axi_wstrb;
	wire mem_axi_bvalid;
	wire mem_axi_bready;
	wire mem_axi_arvalid;
	wire mem_axi_arready;
	wire [31:0] mem_axi_araddr;
	wire [2:0] mem_axi_arprot;
	wire mem_axi_rvalid;
	wire mem_axi_rready;
	wire [31:0] mem_axi_rdata;
	wire [31:0] irq;
	wire [31:0] eoi;

	delay4 #( 1) delay_resetn          (clk, io_resetn         ,    resetn         );
	delay4 #( 1) delay_trap            (clk,    trap           , io_trap           );
	delay4 #( 1) delay_mem_axi_awvalid (clk,    mem_axi_awvalid, io_mem_axi_awvalid);
	delay4 #( 1) delay_mem_axi_awready (clk, io_mem_axi_awready,    me
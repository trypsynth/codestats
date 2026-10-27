// expect: lang="Verilog" total=19 code=14 comment=2 blank=3 shebang=0
// 8-bit counter with synchronous reset.
module counter (
    input  wire       clk,
    input  wire       rst,
    output reg  [7:0] count
);

    wire enable;
    assign enable = 1'b1;

    always @(posedge clk) begin
        if (rst)
            count <= 8'd0;
        else if (enable)
            count <= count + 1;
    end

endmodule

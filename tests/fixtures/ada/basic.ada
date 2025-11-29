-- expect: total=10 code=5 comment=2 blank=3 shebang=0
-- simple ada fixture

with Ada.Text_IO; use Ada.Text_IO;

procedure Main is
begin
	Put_Line("hi");
end Main;

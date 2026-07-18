-- expect: total=9 code=5 comment=2 blank=2 shebang=0
-- simple ada fixture

with Ada.Text_IO; use Ada.Text_IO;

procedure Main is
begin
	Put_Line("hi");
end Main;

-- expect: lang="Ada" total=11 code=8 comment=2 blank=1 shebang=0
-- Print a greeting a few times.
with Ada.Text_IO; use Ada.Text_IO;

procedure Hello is
   Count : constant Integer := 3;
begin
   for I in 1 .. Count loop
      Put_Line ("Hello, world!");
   end loop;
end Hello;

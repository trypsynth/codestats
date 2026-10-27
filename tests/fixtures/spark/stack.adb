-- expect: lang="SPARK" total=14 code=11 comment=2 blank=1 shebang=0
pragma SPARK_Mode (On);

package body Stack with SPARK_Mode is
   -- Push an element; the precondition guarantees there is room.
   procedure Push (S : in out Stack_Type; X : Integer)
     with Pre  => not Is_Full (S),
          Post => Size (S) = Size (S'Old) + 1
   is
   begin
      S.Top := S.Top + 1;
      S.Data (S.Top) := X;
   end Push;
end Stack;

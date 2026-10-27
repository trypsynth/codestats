-- expect: lang="SPARK" total=11 code=8 comment=2 blank=1 shebang=0
package Counter with SPARK_Mode is
   Value : Natural := 0;

   -- Increment the counter without overflowing.
   procedure Increment
     with Global  => (In_Out => Value),
          Depends => (Value => Value),
          Pre     => Value < Natural'Last,
          Post    => Value = Value'Old + 1;
end Counter;

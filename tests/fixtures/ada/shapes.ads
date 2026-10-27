-- expect: lang="Ada" total=10 code=7 comment=2 blank=1 shebang=0
package Shapes is
   -- A simple rectangle record.
   type Rectangle is record
      Width  : Float;
      Height : Float;
   end record;

   function Area (R : Rectangle) return Float;
end Shapes;

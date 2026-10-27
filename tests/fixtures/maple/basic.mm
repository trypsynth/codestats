# expect: lang="Maple" total=12 code=9 comment=2 blank=1 shebang=0
# Compute a Taylor series approximation.
approx := proc(f, x, n)
  local s;
  s := taylor(f, x = 0, n);
  return convert(s, polynom);
end proc;

Shapes := module()
  export area;
  area := r -> Pi * r^2;
end module;

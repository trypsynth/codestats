// expect: lang="Yorick" total=17 code=11 comment=3 blank=3 shebang=0
/* Simple numerical helpers for Yorick. */
require, "string.i";

struct Point {
  double x, y;
}

func norm(p)
{
  local r;
  r = sqrt(p.x^2 + p.y^2);
  return r;
}

// Example usage.
extern origin;

-- expect: lang="Terra" total=18 code=12 comment=2 blank=4 shebang=0
-- Terra function compiled from Lua.
local C = terralib.includec("stdio.h")

struct Point {
  x : int
  y : int
}

terra add(a : int, b : int) : int
  return a + b
end

terra main()
  C.printf("%d\n", add(2, 3))
end

main()

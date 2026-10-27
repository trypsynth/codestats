-- expect: lang="Teal" total=18 code=13 comment=2 blank=3 shebang=0
-- A typed record in Teal.
local record Point
   x: number
   y: number
end

local enum Color
   "red"
   "green"
end

local function dist(p: Point): number
   return math.sqrt(p.x * p.x + p.y * p.y)
end

global origin: Point = { x = 0, y = 0 }
print(dist(origin))

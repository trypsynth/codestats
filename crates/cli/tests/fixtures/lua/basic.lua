-- expect: total=13 code=4 comment=6 blank=3 shebang=0
-- simple lua fixture

--[[
 multi
 line comment
--]]

local function add(a, b)
	return a + b
end

print(add(2, 3))

-- expect: lang="Lua" total=11 code=8 comment=3 blank=0 shebang=0
local url = "http://example.com"
local pattern = '--not a comment'
local long = [[
-- still inside a long string
]]
--[[ a real
block comment ]]
local leveled = [=[
  [[''], {'a': 1}]
]=]

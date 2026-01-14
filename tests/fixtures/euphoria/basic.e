-- expect: total=9 code=5 comment=2 blank=2 shebang=0
-- simple euphoria fixture

function greet(sequence name)
	return "hi " & name
end function

a = greet("world")
puts(1, a)

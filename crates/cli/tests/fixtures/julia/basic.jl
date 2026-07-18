# expect: total=9 code=4 comment=3 blank=2 shebang=0
# simple julia fixture

function greet(name)
	return "hi $(name)"
end

# call it
println(greet("world"))

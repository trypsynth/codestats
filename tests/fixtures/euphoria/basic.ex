-- expect: lang="Euphoria" total=15 code=10 comment=2 blank=3 shebang=0
-- Print the squares of the first few integers.
include std/io.e

function square(integer x)
	return x * x
end function

procedure show(integer n)
	for i = 1 to n do
		printf(1, "%d\n", square(i))
	end for
end procedure

show(5)

% expect: total=9 code=5 comment=2 blank=2 shebang=0
% simple oz fixture

declare
fun {Greet Name}
	"hi " # Name
end

{Browse {Greet "world"}}

-- expect: total=10 code=5 comment=2 blank=3 shebang=0
-- simple elm fixture

module Sample exposing (greet)

greet name =
	"hi " ++ name

main =
	text (greet "world")

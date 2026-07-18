// expect: total=10 code=5 comment=2 blank=3 shebang=0
// simple F# fixture

module Sample

let greet name =
	"hi " + name

let message = greet "world"
printfn "%s" message

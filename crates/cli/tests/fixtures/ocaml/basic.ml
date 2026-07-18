(* expect: total=10 code=5 comment=2 blank=3 shebang=0
simple OCaml fixture *)

let greet name =
	"hi " ^ name

let message = greet "world"
let () = print_endline message

let _ = 1 + 1

// expect: total=10 code=5 comment=2 blank=3 shebang=0
// simple alloy fixture

sig Person {}

fact {
	#Person >= 0
}

run {}

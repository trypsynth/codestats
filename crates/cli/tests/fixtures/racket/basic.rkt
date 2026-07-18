; expect: total=12 code=6 comment=3 blank=3 shebang=0
; simple racket fixture

#lang racket

(define (greet name)
	(string-append "hi " name))

; call it
(define msg (greet "world"))
(displayln msg)
(void)

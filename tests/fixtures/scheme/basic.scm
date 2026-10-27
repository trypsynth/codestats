; expect: total=10 code=5 comment=3 blank=2 shebang=0
; simple scheme fixture

#| block comment |#

(define (add a b) (+ a b))
(define x (add 3 4))
(define y (add x 1))
(display y)
(newline)

; expect: total=9 code=3 comment=3 blank=3 shebang=0
; simple clojure fixture

(ns fixtures.core)

(defn add [a b]
	(+ a b))

; trailing comment

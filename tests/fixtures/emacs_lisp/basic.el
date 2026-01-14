; expect: total=10 code=5 comment=2 blank=3 shebang=0
; Simple Emacs Lisp fixture

(defun greet (name)
	(concat "hi " name))

(setq message (greet "world"))
(message "%s" message)

(setq x 1)

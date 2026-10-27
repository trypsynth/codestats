; expect: lang="Common Lisp" total=17 code=10 comment=5 blank=2 shebang=0
;;; Simple utilities.
(defpackage :utils
  (:use :cl))
(in-package :utils)

#|
Compute the factorial of N recursively.
|#
(defun factorial (n)
  (if (<= n 1)
      1
      (* n (factorial (- n 1)))))

(defvar *greeting* "hello")
(let ((x 10))
  (setf x (factorial x)))

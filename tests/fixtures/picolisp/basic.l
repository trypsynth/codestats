# expect: lang="PicoLisp" total=14 code=9 comment=2 blank=3 shebang=0
# Simple PicoLisp functions.
(de square (N)
   (* N N) )

(de sum-squares (Lst)
   (apply + (mapcar square Lst)) )

(class +Point)
(dm T (X Y)
   (=: x X)
   (=: y Y) )

(println (sum-squares (1 2 3)))

|| expect: lang="Miranda" total=12 code=7 comment=2 blank=3 shebang=0
|| Simple list functions in Miranda.
square :: num -> num
square x = x * x

sumsq :: [num] -> num
sumsq xs = sum (map square xs)

fact n = 1, if n = 0
       = n * fact (n - 1), otherwise

main = show (sumsq [1..10]) where limit = 10

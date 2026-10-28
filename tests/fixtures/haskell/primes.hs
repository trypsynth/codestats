-- expect: lang="Haskell" total=6 code=4 comment=2 blank=0 shebang=0
f x' = x' + 1
g = "{- not a comment"
h = 'a'
-- don't stop
main = print (f 1)

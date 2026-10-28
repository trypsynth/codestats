\* expect: lang="TLA+" total=8 code=5 comment=2 blank=1 shebang=0
---- MODULE Counter ----
\* A counter that only grows.
EXTENDS Naturals
VARIABLE x

Next == x' = x + 1
====

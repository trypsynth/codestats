% expect: lang="Turing" total=15 code=11 comment=2 blank=2 shebang=0
% Ask for a number and count down from it.
var n : int
put "Enter a number: " ..
get n

loop
    exit when n <= 0
    put n
    n := n - 1
end loop

proc greet (name : string)
    put "Hello, ", name
end greet

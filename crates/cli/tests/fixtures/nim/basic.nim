# expect: total=9 code=4 comment=3 blank=2 shebang=0
# simple nim fixture

proc greet(name: string): string =
  "hi " & name

# call it
let msg = greet("world")
echo msg

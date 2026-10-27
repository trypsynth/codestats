; expect: lang="Rebol" total=13 code=9 comment=2 blank=2 shebang=0
REBOL [
    Title: "Greeter"
    Date: 1-Jan-2024
]

; Print a greeting for each name.
greet: func [name [string!]] [
    print ["Hello," name]
]

names: ["Ann" "Bob"]
foreach name names [greet name]

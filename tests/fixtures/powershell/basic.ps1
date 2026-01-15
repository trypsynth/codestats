# expect: total=10 code=5 comment=3 blank=2 shebang=0
# simple powershell fixture

function Greet($Name) {
	"hi $Name"
}

# call it
$msg = Greet "world"
Write-Output $msg

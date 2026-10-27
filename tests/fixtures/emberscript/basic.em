# expect: total=10 code=5 comment=2 blank=3 shebang=0
# simple emberscript fixture

class Greeter
	constructor: (@name) ->

greet = (name) ->
	"hi #{name}"

Greeter

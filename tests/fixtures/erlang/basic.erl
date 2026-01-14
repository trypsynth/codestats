% expect: total=10 code=5 comment=2 blank=3 shebang=0
% simple erlang fixture

-module(sample).
-export([greet/1]).

greet(Name) ->
	"hi " ++ Name.

sample:greet("world").

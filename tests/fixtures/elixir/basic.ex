# expect: total=9 code=5 comment=2 blank=2 shebang=0
# simple elixir fixture

defmodule Sample do
	def greet(name), do: "hi #{name}"
end

greeting = Sample.greet("world")
IO.puts(greeting)

-- expect: lang="Eiffel" total=25 code=18 comment=1 blank=6 shebang=0
class
	ACCOUNT

create
	make

feature -- Access

	balance: INTEGER

feature -- Element change

	deposit (amount: INTEGER)
		require
			positive: amount > 0
		do
			balance := balance + amount
		ensure
			increased: balance = old balance + amount
		end

invariant
	non_negative: balance >= 0
end

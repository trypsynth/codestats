// expect: total=13 code=8 comment=3 blank=2 shebang=0
// typescript interface fixture

interface User {
	id: number;
	name: string;
	email: string;
}

// factory function
function makeUser(id: number, name: string, email: string): User {
	return { id, name, email };
}

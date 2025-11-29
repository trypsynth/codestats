-- expect: total=13 code=4 comment=5 blank=4 shebang=0
-- simple sql fixture

/* multi-line
   comment */

CREATE TABLE users (
	id INTEGER PRIMARY KEY,
	name TEXT
);

-- trailing comment

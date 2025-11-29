// expect: total=14 code=6 comment=4 blank=4 shebang=0
// simple c++ fixture

#include <iostream>

/* block comment start
still comment */

int main() {
	int x = 1; // inline comment
	std::cout << "hi" << std::endl;
	return x;
}

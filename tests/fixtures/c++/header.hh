// expect: lang="C++" total=10 code=7 comment=1 blank=2 shebang=0
#pragma once

#include <string>

class Greeter {
public:
	std::string greet() const;
};
inline int answer() { return 42; }

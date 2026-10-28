// expect: lang="C++" total=8 code=6 comment=1 blank=1 shebang=0
#ifndef COMMON_FLAG
#error "Define COMMON_FLAG before including this file"
#endif

namespace __sanitizer {
COMMON_FLAG(bool, symbolize, true, "If set, symbolize reports.")
}

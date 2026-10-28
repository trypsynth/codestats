// expect: lang="Objective-C" total=13 code=9 comment=2 blank=2 shebang=0
#ifdef __cplusplus
# error "A C++ compiler has been selected for Objective-C."
#endif

/* Plain C compiled as Objective-C. */
#include "abi.h"

int main(int argc, char *argv[])
{
  (void)argv;
  return argc;
}

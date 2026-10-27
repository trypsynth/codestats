// expect: lang="Objective-C" total=17 code=14 comment=1 blank=2 shebang=0
#import "Car.h"
#include <stdio.h>

@implementation Car
- (instancetype)initWithModel:(NSString *)model {
	if ((self = [super init])) {
		_model = [model copy];
	}
	return self;
}

- (void)drive {
	printf("%s is driving\n", [self.model UTF8String]);
	NSLog(@"Speed: %ld", (long)42);
}
@end

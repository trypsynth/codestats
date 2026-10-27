// expect: lang="Objective-C++" total=19 code=15 comment=2 blank=2 shebang=0
#import <Foundation/Foundation.h>
#include <vector>

// Bridge a C++ vector to an NSArray.
template <typename T>
NSArray *toArray(const std::vector<T> &values) {
	NSMutableArray *result = [NSMutableArray array];
	for (const auto &v : values) {
		[result addObject:@(v)];
	}
	return result;
}

@implementation Bridge
- (NSArray *)numbers {
	return toArray(std::vector<int>{1, 2, 3});
}
@end

// expect: lang="Objective-C" total=12 code=8 comment=1 blank=3 shebang=0
#import <Foundation/Foundation.h>

@class Engine;

@interface Car : NSObject
@property (nonatomic, copy) NSString *model;
@property (nonatomic, strong) Engine *engine;

- (instancetype)initWithModel:(NSString *)model;
- (void)drive;
@end

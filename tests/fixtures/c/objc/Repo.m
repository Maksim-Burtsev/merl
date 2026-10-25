#import "Repo.h"

@implementation UserRepository
- (User *)findUserWithID:(NSString *)userID inContext:(Context *)ctx {
    return [self lookup:userID];
}

+ (instancetype)sharedRepository {
    return nil;
}
@end

@implementation NSString (Slug)
- (NSString *)slug {
    return self;
}
@end

@interface UserRepository ()
@property (nonatomic) NSString *token;
@end

@implementation UserRepository (Token)
- (NSString *)peek {
    return self.token;
//              ^ d: objc/Repo.m:20
}
@end

@implementation Timer
- (void)waitFor:(NSInteger)ticks then:(Done)completion {
//                                          ^ d: objc/Repo.m:31
}
@end

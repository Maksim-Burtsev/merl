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

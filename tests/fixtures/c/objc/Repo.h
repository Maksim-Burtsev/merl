#import <Foundation/Foundation.h>

@class Session;
@protocol Store;

@protocol Repo <NSObject>
- (User *)findUserWithID:(NSString *)userID inContext:(Context *)ctx;
@end

@protocol Plain
@end

typedef NS_ENUM(NSInteger, Status) {
    StatusOpen,
    StatusClosed,
};

typedef NS_OPTIONS(NSUInteger, Flags) {
    FlagsNone = 0,
};

typedef NS_CLOSED_ENUM(NSInteger, Shade) { ShadeDark };
typedef NS_ERROR_ENUM(RepoErrorDomain, RepoError) { RepoErrorGone = 1 };

@interface Root
@end

@interface UserRepository : NSObject <Repo> {
    NSString *_cache;
}
@property (nonatomic, copy) NSString *baseURL;
@property (nonatomic, copy) void (^completion)(NSError *error);
+ (instancetype)sharedRepository;
- (User *)findUserWithID:(NSString *)userID inContext:(Context *)ctx;
@end

@interface Mirror <Repo>
@end

@interface NSString (Slug)
- (NSString *)slug;
@end

@interface UserRepository ()
@property (nonatomic) NSInteger hits;
@end

/*
@interface Ghost : NSObject
- (void)haunt;
@property (nonatomic) int ghostly;
*/

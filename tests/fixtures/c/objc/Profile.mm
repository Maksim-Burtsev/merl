#import "Repo.h"

@implementation ProfileController
//              ^ d: none
- (void)load {
    User *user = [self.repository findUserWithID:@"42" inContext:nil];
//                                ^ d: picker objc/Repo.h:7, objc/Repo.h:34, objc/Repo.m:4
//                                                     ^ d: picker objc/Repo.h:7, objc/Repo.h:34, objc/Repo.m:4
//                     ^ d: none
    NSLog(@"%@ %@", user, self.repository.baseURL);
//                                        ^ d: objc/Repo.h:31
    self.completion = nil;
//       ^ d: objc/Repo.h:32
    self.hits = [UserRepository sharedRepository].hits;
//       ^ d: objc/Repo.h:45
//                              ^ d: picker objc/Repo.h:33, objc/Repo.m:8
//               ^ d: objc/Repo.h:28
    SEL s = @selector(slug);
//                    ^ d: picker objc/Repo.h:41, objc/Repo.m:14
    Status st = StatusOpen;
//  ^ d: objc/Repo.h:13
//              ^ d: none
    Flags f = 0; Shade sh; RepoError e;
//  ^ d: objc/Repo.h:18
//               ^ d: objc/Repo.h:22
//                         ^ d: objc/Repo.h:23
    id<Repo> r = nil; id<Plain> p; Root *root; Mirror *m;
//     ^ d: objc/Repo.h:6
//                       ^ d: objc/Repo.h:10
//                                 ^ d: objc/Repo.h:25
//                                             ^ d: objc/Repo.h:37
    Session *session; id<Store> store; Ghost *ghost;
//  ^ d: none
//                       ^ d: none
//                                     ^ d: none
    [self haunt]; self.ghostly = _cache;
//        ^ d: none
//                     ^ d: none
//                               ^ d: none
}
@end

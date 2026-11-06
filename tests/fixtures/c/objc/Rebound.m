int crumb;

void crumbForIn(NSArray *xs) { for (id crumb in xs) use(crumb); use(crumb); }
//                                                      ^ d: objc/Rebound.m:3
//                                                                  ^ d: objc/Rebound.m:1
void crumbForInTyped(NSArray *xs) { for (NSString *crumb in xs) [crumb length]; }
//                                                               ^ d: objc/Rebound.m:6
void crumbForInBare(NSArray *xs) { for (crumb in xs) use(crumb); }
//                                                       ^ d: objc/Rebound.m:1
void crumbForInLater(NSArray *xs) { use(crumb); for (id crumb in xs) use(crumb); }
//                                      ^ d: objc/Rebound.m:1
void crumbForInClosed(NSArray *xs) { for (id crumb in xs) {} use(crumb); }
//                                                               ^ d: objc/Rebound.m:1
void crumbBlock(void) { id b = ^(int crumb) { return crumb; }; use(crumb); }
//                                                   ^ d: objc/Rebound.m:14
//                                                                 ^ d: objc/Rebound.m:1
void crumbBlockSpaced(void) { id b = ^ (int crumb) { return crumb; }; }
//                                                          ^ d: objc/Rebound.m:17
void crumbBlockOver(void) {
    id b = ^(int crumb) {
        return crumb;
        //     ^ d: objc/Rebound.m:20
    };
    use(crumb);
    //  ^ d: objc/Rebound.m:1
}
void crumbBlockReturns(void) {
    id b = ^NSString *(id crumb) {
        return crumb;
        //     ^ d: objc/Rebound.m:28
    };
}
void crumbBlockNested(void) {
    id b = ^(int crumb) {
        id c = ^(int grit) {
            return crumb + grit;
            //     ^ d: objc/Rebound.m:34
        };
    };
}
void crumbBlockArg(NSArray *xs) {
    [xs enumerateObjectsUsingBlock:^(id obj, NSUInteger crumb, BOOL *stop) {
        use(crumb);
        //  ^ d: objc/Rebound.m:42
    }];
    use(crumb);
    //  ^ d: objc/Rebound.m:1
}
void crumbBlockArgLine(NSArray *xs) { [xs enumerateObjectsUsingBlock:^(id obj, NSUInteger crumb, BOOL *stop) { use(crumb); }]; }
//                                                                                                                 ^ d: objc/Rebound.m:49
void crumbBlockBare(void) { id b = ^{ return crumb; }; }
//                                           ^ d: objc/Rebound.m:1
void crumbBlockOther(void) { id b = ^(int grit) { return crumb; }; }
//                                                       ^ d: objc/Rebound.m:1
void crumbBlockLater(void) { use(crumb); id b = ^(int crumb) { return crumb; }; }
//                               ^ d: objc/Rebound.m:1
void crumbBlockType(void) { void (^b)(int crumb) = nil; use(crumb); }
//                                                          ^ d: objc/Rebound.m:1
void crumbString(void) { NSLog(@"^(int crumb) {"); use(crumb); }
//                                                     ^ d: objc/Rebound.m:1
void crumbComment(void) { /* ^(int crumb) { */ use(crumb); }
//                                                 ^ d: objc/Rebound.m:1
void crumbXor(int a) { if ((a ^ (crumb))) { use(crumb); } }
//                                              ^ d: objc/Rebound.m:1

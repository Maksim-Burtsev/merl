// Every construct `f` folds in Objective-C on top of C's, and the cases that could break it.
#import <Foundation/Foundation.h>  // f: 2-4

#import "Box.h"  // f: 2-4
@class Lid;  // f: none
@protocol Opening;  // f: none
@protocol Opening <NSObject>  // f: 7-11
- (void)open;  // f: 7-11
- (void)openWith:(NSString *)key  // f: 9-10
          inside:(Lid *)lid;  // f: 7-11
@end  // f: 7-11

API_AVAILABLE(ios(13.0))  // f: 13-19
@interface Box : NSObject <Opening> {  // f: 13-19
    int _count;  // f: 13-19
}  // f: 13-19
@property (nonatomic, strong,  // f: 17-18
          nullable) NSString *name;  // f: 13-19
@end  // f: 13-19

@implementation Box  // f: 21-64

- (void)open {  // f: 23-56
    NSArray *items = @[  // f: 24-26
        @"a",  // f: 23-56
    ];  // f: 23-56
    NSDictionary *map = @{  // f: 27-29
        @"b" : @1,  // f: 23-56
    };  // f: 23-56
    [self each:^(NSString *item) {  // f: 30-32
        NSLog(@"%@ {", item);  // f: 23-56
    }];  // f: 23-56
    dispatch_async(queue, ^{  // f: 33-35
        [self close];  // f: 23-56
    });  // f: 23-56
    @try {  // f: 36-44
        [self go];  // f: 23-56
    } @catch (NSException *e) {  // f: 38-41
        @throw [NSException exceptionWithName:@"x"  // f: 39-41
                                       reason:nil  // f: 23-56
                                     userInfo:nil];  // f: 23-56
    } @finally {  // f: 42-44
        [self close];  // f: 23-56
    }  // f: 23-56
    NSData *(^load)(void) = ^NSData *{  // f: 45-47
        return nil;  // f: 23-56
    };  // f: 23-56
    @autoreleasepool {  // f: 48-50
        [self go];  // f: 23-56
    }  // f: 23-56
    @synchronized (self) {  // f: 23-56
        if (_count) {  // f: 52-54
            _count = 0;  // f: 23-56
        }  // f: 23-56
    }  // f: 23-56
}  // f: 23-56

- (void)openWith:(NSString *)key  // f: 58-62
          inside:(Lid *)lid  // f: 58-62
{  // f: 58-62
    Protocol *p = @protocol(Opening);  // f: 58-62
}  // f: 58-62

@end  // f: 21-64

static void after(void) {  // f: 66-68
    go();  // f: 66-68
}  // f: 66-68

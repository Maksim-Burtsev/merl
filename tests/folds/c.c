// Every construct `f` folds in C, and the cases that once broke it.
#include <stdio.h>
/* a comment ends a run of includes */
#include "a.h"  // f: 4-6

#include "b.h"  // f: 4-6
#ifndef X  // f: 7-15
#define X 1  // f: 7-15
#elif defined(Y)  // f: 9-14
#define Y 2  // f: 9-14

#else  // f: 12-14
#define Z 3 /* a comment that goes  // f: 13-14
                on */  // f: 13-14
#endif  // f: 7-15
#define F(a) \
    do { \
        a; \
    } while (0)  // f: 16-19
/*  // f: 20-22
 * doc  // f: 20-22
 */  // f: 20-22
static int  // f: 23-53
foo(int a)  // f: 23-53
{  // f: 23-53
    int b[] = {  // f: 26-28
        1,  // f: 23-53
    };  // f: 23-53
    {  // f: 29-31
        b[0] = 2;  // f: 23-53
    }  // f: 23-53
    if (a)  // f: 32-35
        return 1;  // f: 23-53
    else  // f: 23-53
        return 2;  // f: 23-53
    switch (a) {  // f: 36-46
    case 1:  // f: 37-39
        a++;  // f: 23-53
        break;  // f: 23-53

    case 2: {  // f: 41-43
        a--;  // f: 23-53
    }  // f: 23-53
    default:  // f: 44-45
        break;  // f: 23-53
    }  // f: 23-53
    do {  // f: 47-49
        a++;  // f: 23-53
    } while (a < 3);  // f: 23-53
    return (struct s){  // f: 50-52
        1,  // f: 23-53
    }.x;  // f: 23-53
}  // f: 23-53
const char *names[] = {  // f: 54-57
    "a {",  // f: 54-57
    "b",  // f: 54-57
};  // f: 54-57

struct pt {  // f: 59-60
    int x;  // f: 59-60
} pts[] = {  // f: 61-68
    {  // f: 62-64
        1,  // f: 62-64
    },  // f: 62-64
    {  // f: 65-67
        2,  // f: 65-67
    },  // f: 65-67
};  // f: 61-68

union u {
    int i;
    float f;
};

typedef struct {  // f: 75-77
    int a;  // f: 75-77
} T;  // f: 75-77
#include "z.h"  // f: none
#import "y.h"  // f: none

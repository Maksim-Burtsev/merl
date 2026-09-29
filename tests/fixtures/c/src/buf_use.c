#include "shop/buf.h"

int buf_size(struct buf_s *b)
{
    int n = b->len;
    //         ^ d: include/shop/buf.h:7
    n += b->data != 0;
    //      ^ d: include/shop/buf.h:6
    n += b->cap + b->head;
    //      ^ d: include/shop/buf.h:8
    //               ^ d: include/shop/buf.h:10
    n += b->next != 0;
    //      ^ d: include/shop/buf.h:9
    n += b->u.mark + b->sealed;
    //      ^ d: include/shop/buf.h:15
    //        ^ d: include/shop/buf.h:13
    //                  ^ d: include/shop/buf.h:16
    n += b->bytes[0] + b->flush(b);
    //      ^ d: include/shop/buf.h:19
    //                    ^ d: include/shop/buf.h:11
    return n;
}

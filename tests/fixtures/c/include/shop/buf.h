#ifndef SHOP_BUF_H
#define SHOP_BUF_H

/* A buffer's bytes, laid out as { header } then the data section. */
struct buf_s {
    char *data;
    int len;
    int cap;
    struct buf_s *next;
    int head;
    int (*flush)(struct buf_s *b);
    union {
        int mark;
        char sign;
    } u;
    unsigned sealed : 1;
    /* The bytes follow the header { as rax.h has them }:
     * [header][abc] */
    unsigned char bytes[];
};

#endif

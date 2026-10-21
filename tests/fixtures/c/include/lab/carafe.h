#ifndef LAB_CARAFE_H
#define LAB_CARAFE_H

struct carafe {
    int volume;
    struct carafe *next;
    int (*swirl)(struct carafe *c);
    union {
        int ml;
        float oz;
    };
};

struct beaker {
    struct carafe base;
    int brim;
};

struct dial {
    int tm_hour;
    long tv_sec;
    int quot;
};

struct burner {
    int flame;
};

typedef struct carafe carafe_t;
typedef struct beaker Beaker;
typedef int (*swirl_fn)(struct carafe *c);

struct carafe *carafe_new(int volume);
struct beaker *beaker_of(struct carafe *c);
int swirl(struct carafe *c);
int burner(int flame);

extern int volume;

#define BRIM(b) ((b)->brim)
#define DECLARE_SWIRL(name) int name(struct carafe *c)

DECLARE_SWIRL(swirl_twice);

#endif

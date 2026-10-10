struct drum {
    int skin;
};

int beat(struct drum *d) { return d->skin; }
//                                   ^ d: src/drum.c:2

struct pail_s {
    int handle;
};

struct jug {
    int handle;
};

typedef struct pail_s pail;

int carry(pail *p) { return p->handle; }
//                             ^ d: src/drum.c:9

struct jar {
    int fill;
    int (*pour)(int);
};

int use_jar(void) { return get()->fill(1) + get()->pour(2); }
//                                ^ d: none
//                                                 ^ d: src/drum.c:23

int tip(void) { return pail->handle + jar->fill; }
//                     ^ d: none
//                                    ^ d: none

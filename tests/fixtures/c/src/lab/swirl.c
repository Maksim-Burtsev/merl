#include "lab/lab.h"

#include <stdlib.h>
#include <sys/time.h>
#include <time.h>

static const char *recipe = "struct carafe { int volume; }; int swirl(struct carafe *c) {";

/*
int swirl(struct carafe *c) {
struct beaker {
*/

#if 0
int burner(int flame) { return 0; }
struct burner { int flame; };
#endif

static int decant(void) { return 1; }

int outside(void)
{
    time_t now = time(0);
    struct tm *t = localtime(&now);
    div_t d = div(7, 2);
    struct timeval tv;
    return t->tm_hour + d.quot + (int)tv.tv_sec;
    //        ^ d: none
    //                    ^ d: none
    //                                   ^ d: none
}

int stamped(struct dial *s)
{
    return s->tm_hour + s->quot;
    //        ^ d: include/lab/carafe.h:20
    //                     ^ d: include/lab/carafe.h:22
}

int chains(struct carafe *c)
{
    int n = carafe_new(1)->next->volume + beaker_of(c)->base.volume + beaker_of(c)->brim;
    //                     ^ d: include/lab/carafe.h:6
    //                           ^ d: include/lab/carafe.h:5
    //                                                  ^ d: include/lab/carafe.h:15
    //                                                       ^ d: include/lab/carafe.h:5
    //                                                                              ^ d: include/lab/carafe.h:16
    return n + c
        ->next
        ->volume;
    //    ^ d: include/lab/carafe.h:5
}

int calls(struct carafe *c)
{
    return c->swirl(c) + swirl(c) + swirl_twice(c) + decant() + c->ml;
    //        ^ d: include/lab/carafe.h:7
    //                   ^ d: src/lab/funnel.c:11
    //                              ^ d: src/lab/funnel.c:13
    //                                               ^ d: src/lab/swirl.c:19
    //                                                             ^ d: include/lab/carafe.h:9
}

int shadows(struct beaker *c, struct carafe *head)
{
    int total = 0;
    for (struct carafe *c = head; c; c = c->next)
        total += c->volume;
        //          ^ d: include/lab/carafe.h:5
    {
        struct carafe *c = head;
        total += c->ml;
        //          ^ d: include/lab/carafe.h:9
    }
    struct carafe *carafe = carafe_new(2);
    //                      ^ d: src/lab/funnel.c:7
    int brim = BRIM(c);
    //         ^ d: include/lab/carafe.h:40
    return total + brim + c->brim + carafe->volume;
    //             ^ d: src/lab/swirl.c:77
    //                       ^ d: include/lab/carafe.h:16
    //                              ^ d: src/lab/swirl.c:75
    //                                      ^ d: include/lab/carafe.h:5
}

int aliases(carafe_t *k, Beaker bk)
{
    swirl_fn f = swirl;
  //^ d: include/lab/carafe.h:31
    return k->volume + bk.brim + bk.base.oz + f(k) + volume;
    //        ^ d: include/lab/carafe.h:5
    //                    ^ d: include/lab/carafe.h:16
    //                                   ^ d: include/lab/carafe.h:10
    //                                        ^ d: src/lab/swirl.c:88
    //                                               ^ d: src/lab/funnel.c:3
}

int tags(void)
{
    struct burner b = { .flame = 3 };
    //     ^ d: picker src/lab/swirl.c:106, include/lab/carafe.h:25, include/lab/carafe.h:36, src/lab/funnel.c:15; want include/lab/carafe.h:25 (#1205)
    //                   ^ d: include/lab/carafe.h:26
    struct carafe c = { .volume = 1, .next = 0 };
    //                   ^ d: include/lab/carafe.h:5
    //                                ^ d: picker include/lab/carafe.h:6, include/shop/buf.h:9; want include/lab/carafe.h:6 (#1206)
    return burner(b.flame) + (struct beaker){ .brim = 2 }.brim + c.volume + (int)sizeof(struct burner) + (int)(recipe != 0);
    //     ^ d: picker include/lab/carafe.h:25, include/lab/carafe.h:36, src/lab/funnel.c:15; want src/lab/funnel.c:15 (#1205)
    //              ^ d: include/lab/carafe.h:26
    //                                         ^ d: include/lab/carafe.h:16
    //                                                    ^ d: include/lab/carafe.h:16
    //                                                                                         ^ d: picker include/lab/carafe.h:25, include/lab/carafe.h:36, src/lab/funnel.c:15; want include/lab/carafe.h:25 (#1205)
}

int labels(struct beaker *b)
{
    int brim = 0;
    if (b->brim)
        goto brim;
        //   ^ d: src/lab/swirl.c:121
    return 1;
brim:
    return brim;
    //     ^ d: src/lab/swirl.c:116
}

/* A small ledger: entries, totals and a report. */
#include <stdio.h>
#include <stdlib.h>
#include "ledger.h"

#define MAX_ENTRIES 64
#define CENTS(x) ((long)((x) * 100))
#ifndef LEDGER_DEBUG
#  define LEDGER_DEBUG 0
#endif

typedef enum { CREDIT, DEBIT = 2 } kind_t;

typedef struct entry {
    const char *memo;
    long amount;
    kind_t kind;
    unsigned int flags : 3;
    struct entry *next;
} entry_t;

union value {
    int i;
    double d;
};

static entry_t entries[MAX_ENTRIES];
static size_t count = 0;
extern int verbose;
volatile sig_atomic_t stop = 0;

typedef int (*compare_fn)(const void *, const void *);

static int by_amount(const void *a, const void *b)
{
    const entry_t *x = a, *y = (const entry_t *)b;
    return (x->amount > y->amount) - (x->amount < y->amount);
}

int ledger_add(const char *memo, double value, kind_t kind)
{
    if (count >= MAX_ENTRIES || memo == NULL) {
        return -1;
    }
    entry_t *e = &entries[count++];
    e->memo = memo;
    e->amount = kind == DEBIT ? -CENTS(value) : CENTS(value);
    e->kind = kind;
    e->next = NULL;
    return 0;
}

long ledger_total(void)
{
    long sum = 0;
    for (size_t i = 0; i < count; i++) {
        sum += entries[i].amount;
    }
    return sum;
}

void ledger_report(FILE *out)
{
    char line[80];
    qsort(entries, count, sizeof(entry_t), by_amount);
    for (size_t i = 0; i < count; ++i) {
        snprintf(line, sizeof line, "%-20s %8.2f\n", entries[i].memo, entries[i].amount / 100.0);
        fputs(line, out);
    }
    fprintf(out, "total: %ld (%s)\n", ledger_total(), count ? "ok" : "empty");
    printf("done\t%c %x %o\n", 'x', 0xFF, 017);
}

int main(int argc, char **argv)
{
    int status = EXIT_SUCCESS;
    unsigned char mask = 0x0f & ~0x03;
    float rate = 1.5e-3f;
    _Bool ok = true;

    if (argc < 2) {
        goto usage;
    }
    switch (argv[1][0]) {
    case 'a':
        ledger_add(argv[2], atof(argv[3]), CREDIT);
        break;
    case 'r':
        ledger_report(stdout);
        break;
    default:
        status = EXIT_FAILURE;
    }
    while (!stop && mask >>= 1) {
        rate *= 2;
    }
    do {
        ok = !ok;
    } while (ok);
    return status;

usage:
    fprintf(stderr, "usage: %s a MEMO AMOUNT | r\n", argv[0]);
    return 2;
}

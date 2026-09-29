int link(const char *from, const char *to);
struct node_s { int v; };
int mask = 4;
int grain = 1;

int link_value(struct node_s *link) { return link->v; }
//                                           ^ d: src/locals.c:6

int check(int x)
{
    x & mask;
    x && grain == 1;
    const char *s = "{ int mask; } int grain;";
    return (x & mask) + grain + (s != 0);
    //          ^ d: src/locals.c:3
    //                  ^ d: src/locals.c:4
}

int tally_up(int n)
{
    {
        int mask = n;
    }
    return mask;
    //     ^ d: src/locals.c:3
}

int sum_to(int n)
{
    int total = 0;
    for (int step = 0; step < n; step++) {
        total += step;
      //^ d: src/locals.c:30
        //       ^ d: src/locals.c:31
    }
    return total;
}

int shadow(int mask)
{
    if (mask) {
        int mask = 1;
        return mask;
        //     ^ d: src/locals.c:42
    }
    return mask;
    //     ^ d: src/locals.c:39
}

int split(struct node_s *link, int grain)
{
    int a = grain, *mask = &a;
    struct node_s copy = *link;
    return *mask + copy.v;
    //      ^ d: src/locals.c:52
    //             ^ d: src/locals.c:53
}

struct herd { int size; };
struct herd *herd;
int herd_size(void)
{
    return herd->size;
    //     ^ d: src/locals.c:60
}

int assigned(int grain)
{
    int total;
    total = link_value(0);
    //      ^ d: src/locals.c:6
    return total;
    //     ^ d: src/locals.c:69
}

int braceless(const int *row, int n)
{
    int sum = 0;
    for (int col = 0; col < n; col++)
        sum += row[col];
        //         ^ d: src/locals.c:79
    return sum;
}

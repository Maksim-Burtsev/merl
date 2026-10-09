struct lid {
    int seal;
};

struct base {
    int seal;
};

struct lid *bend(void);

struct base *bend(void) { return 0; }

int sealed(void) { return bend()->seal; }
//                                ^ d: !jump

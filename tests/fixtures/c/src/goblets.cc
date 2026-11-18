struct Goblet {
    void quaff();
};

template <typename Goblet> struct Rack {
    Goblet held;
    //^ d: src/goblets.cc:5
    void goblet_fill(Goblet x) { x.quaff(); }
    //               ^ d: src/goblets.cc:5
    //                             ^ d: none
};

template <class Goblet> void goblet_pour(Goblet x) {}
//                                       ^ d: src/goblets.cc:13
void goblet_plain(Goblet x) { x.quaff(); }
//                ^ d: src/goblets.cc:1
//                              ^ d: src/goblets.cc:2

template <typename Goblet>
void goblet_wrapped(Goblet x) {}
//                  ^ d: src/goblets.cc:19

template <typename T> void goblet_t(T x) { x.quaff(); }
//                                           ^ d: src/goblets.cc:2
template <typename Goblet> void goblet_free(Goblet x) { x.quaff(); }
//                                                        ^ d: none
template <typename Goblet> void goblet_ptr(Goblet* x) { x->quaff(); }
//                                                         ^ d: none
template <typename Goblet> void goblet_ref(const Goblet& x) { x.quaff(); }
//                                                              ^ d: none
template <typename Goblet>
void goblet_lines(Goblet x) {
    x.quaff();
    //^ d: none
    Goblet y;
    y.quaff();
    //^ d: none
}
template <typename Goblet> struct Rack2 {
    Goblet held;
    void f() { held.quaff(); }
    //              ^ d: none
};
template <typename T> struct Rack3 {
    T held;
    void f() { this->held.quaff(); }
    //                    ^ d: src/goblets.cc:2
};
template <typename Goblet> struct Rack4 {
    Goblet held;
    void f() { this->held.quaff(); }
    //                    ^ d: none
};
template <typename T> struct Rack5 {
    T held;
    Goblet cup;
    void f() { cup.quaff(); this->cup.quaff(); }
    //             ^ d: src/goblets.cc:2
    //                                ^ d: src/goblets.cc:2
    void g(Goblet held) { held.quaff(); }
    //                         ^ d: src/goblets.cc:2
};
template <typename T> class Rack6 {
public:
    T* held;
    void f() { held->quaff(); this->held->quaff(); }
    //               ^ d: src/goblets.cc:2; want none (#1572)
};
void goblet_plain_ptr(Goblet* x) { x->quaff(); }
//                                    ^ d: src/goblets.cc:2
template <class Goblet> void goblet_class(Goblet x) { x.quaff(); }
//                                                      ^ d: none
template <class Goblet>
void goblet_class_lines(Goblet x) {
    x.quaff();
    //^ d: none
}
template <typename T>
requires Quaffable<T>
void goblet_required(T x) { x.quaff(); }
//                            ^ d: src/goblets.cc:2
template <Quaffable T> void goblet_concept(T x) { x.quaff(); }
//                                                  ^ d: src/goblets.cc:2
Goblet goblet_global;
template <typename T> void goblet_reads_global(T x) { goblet_global.quaff(); }
//                                                                  ^ d: src/goblets.cc:2
template <typename T> struct Rack7 {
    T* held;
    void f() { held->quaff(); }
    //               ^ d: src/goblets.cc:2
};
struct Hasher {
    virtual unsigned Sum() const = 0;
};
struct Fast : Hasher {
    unsigned Sum() const override { return 1; }
};
template <class H> int crc_pick(H *h) { return h->Sum() ? 1 : 0; }
//                                                ^ d: picker src/goblets.cc:93, src/goblets.cc:96
template <typename Goblet> class Rack8 {
public:
    Goblet* held;
    void f() { held->quaff(); }
    //               ^ d: none
};
template <typename Goblet> class Rack9 {
public:
    Goblet held;
    void f() { held.quaff(); this->held.quaff(); }
    //              ^ d: none
    //                                  ^ d: none
};
template <typename Goblet> class Rack10 {
    Goblet held;
    void f() { held.quaff(); }
    //              ^ d: none
};
struct Flagon {
    void swill();
};
template <typename Goblet> class Chalice {
public:
    Chalice* next;
    void swill();
    void f() { next->swill(); }
    //               ^ d: src/goblets.cc:124
};

struct Goblet {
    void quaff();
};

template <typename Goblet> struct Rack {
    Goblet held;
    //^ d: src/goblets.cc:5
    void goblet_fill(Goblet x) { x.quaff(); }
    //               ^ d: src/goblets.cc:5
    //                             ^ d: src/goblets.cc:2; want none (#1438)
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
//                                           ^ d: src/goblets.cc:2; want none (#1438)
template <typename Goblet> void goblet_free(Goblet x) { x.quaff(); }
//                                                        ^ d: src/goblets.cc:2; want none (#1438)
template <typename Goblet> void goblet_ptr(Goblet* x) { x->quaff(); }
//                                                         ^ d: src/goblets.cc:2; want none (#1438)
template <typename Goblet> void goblet_ref(const Goblet& x) { x.quaff(); }
//                                                              ^ d: src/goblets.cc:2; want none (#1438)
template <typename Goblet>
void goblet_lines(Goblet x) {
    x.quaff();
    //^ d: src/goblets.cc:2; want none (#1438)
    Goblet y;
    y.quaff();
    //^ d: src/goblets.cc:2; want none (#1438)
}
template <typename Goblet> struct Rack2 {
    Goblet held;
    void f() { held.quaff(); }
    //              ^ d: src/goblets.cc:2; want none (#1438)
};
template <typename T> struct Rack3 {
    T held;
    void f() { this->held.quaff(); }
    //                    ^ d: src/goblets.cc:2; want none (#1438)
};
template <typename Goblet> struct Rack4 {
    Goblet held;
    void f() { this->held.quaff(); }
    //                    ^ d: src/goblets.cc:2; want none (#1438)
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
    //               ^ d: src/goblets.cc:2; want none (#1438)
};
void goblet_plain_ptr(Goblet* x) { x->quaff(); }
//                                    ^ d: src/goblets.cc:2

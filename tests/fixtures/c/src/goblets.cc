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
//                                           ^ d: none
template <typename Goblet> void goblet_free(Goblet x) { x.quaff(); }
//                                                        ^ d: none
template <typename Goblet> void goblet_ptr(Goblet* x) { x->quaff(); }
//                                                         ^ d: src/goblets.cc:2; want none (#1438)
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
    //              ^ d: src/goblets.cc:2; want none (#1438)
};

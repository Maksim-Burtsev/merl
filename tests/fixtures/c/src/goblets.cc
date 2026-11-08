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

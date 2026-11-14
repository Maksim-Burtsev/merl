struct Situla {
    void brim();
};

template <typename Situla = int> struct SitulaKeg {
    Situla held;
  //^ d: src/situlas.cc:5
};

template <typename... Situla> void situla_many(Situla... xs) {}
//                                             ^ d: src/situlas.cc:10

template <class Situla, int N> struct SitulaArr {
    Situla items[N];
  //^ d: src/situlas.cc:13
};

template <typename A,
          typename Situla>
struct SitulaPair {
    Situla b;
  //^ d: src/situlas.cc:18
};

template <typename Situla> struct SitulaBin {
    void situla_fill(Situla x);
    //               ^ d: src/situlas.cc:25
    void deep(int n) {
        if (n) {
            Situla y{};
          //^ d: src/situlas.cc:25
            Situla z;
          //^ d: src/situlas.cc:25
        }
        Situla w{};
      //^ d: src/situlas.cc:25
    }
};

template <typename Situla> void SitulaBin<Situla>::situla_fill(Situla x) {}
//                                                             ^ d: src/situlas.cc:40

template <typename T> struct SitulaOuter {
    template <typename Situla> void m(Situla x) {}
    //                                ^ d: src/situlas.cc:44
    T kept;
};

template <template <typename> class Situla> struct SitulaH {
    Situla<int> x;
  //^ d: src/situlas.cc:49
};

template <typename Situla> struct SitulaD : Situla {};
//                                          ^ d: src/situlas.cc:54

template <> struct SitulaKeg<Situla> {};
//                           ^ d: src/situlas.cc:1

void situla_plain(Situla x) {}
//                ^ d: src/situlas.cc:1

template <typename Situla>
    requires true
void situla_req(Situla x) {}
//              ^ d: src/situlas.cc:63

template <typename Situla>
using SitulaAlias = Situla;
//                  ^ d: src/situlas.cc:68

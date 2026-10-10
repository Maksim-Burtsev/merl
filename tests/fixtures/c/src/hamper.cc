class Hamper /* api */ {
  virtual void wrap() = 0;
};

int pack() { return make_hamper()->wrap(); }
//                                 ^ d: src/hamper.cc:2

struct Crate {};
class Basket /* api */
    : public Crate {
  virtual void weave() = 0;
};

int braid() { return make_basket()->weave(); }
//                                  ^ d: src/hamper.cc:11

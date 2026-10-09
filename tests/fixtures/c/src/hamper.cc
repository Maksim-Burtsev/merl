class Hamper /* api */ {
  virtual void wrap() = 0;
};

int pack() { return make_hamper()->wrap(); }
//                                 ^ d: src/hamper.cc:2

namespace quoted {

struct Stylus {
  int blot() const { return 1; }
};

struct Nib {
  int blot() const { return 2; }
};

const char *Plain(const Stylus &s) {
  return "s.blot()";
  //        ^ d: picker src/quoted.cc:4, src/quoted.cc:8
}

const char *Raw(const Stylus &s) {
  return R"x(s.blot())x";
  //           ^ d: picker src/quoted.cc:4, src/quoted.cc:8
}

const char *Closer(const Stylus &s) {
  return R"x()" s.blot() )x";
  //              ^ d: picker src/quoted.cc:4, src/quoted.cc:8
}

const char *Lines(const Stylus &s, const Stylus *p) {
  return R"x(
    s.blot()
    //^ d: picker src/quoted.cc:4, src/quoted.cc:8
    p->blot()
    //  ^ d: picker src/quoted.cc:4, src/quoted.cc:8
)x";
}

int After(const Stylus &s) {
  const char *t = R"x(
)" s.blot())x"; return s.blot() + 1'000;
  //                     ^ d: src/quoted.cc:4
  return 1'000 + s.blot() + t[0];
  //               ^ d: src/quoted.cc:4
}

}  // namespace quoted

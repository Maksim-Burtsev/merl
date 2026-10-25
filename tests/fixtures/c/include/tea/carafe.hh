namespace pot {
struct Carafe {
  explicit Carafe(int ml);
  Carafe(const char* name, int ml);
  Carafe();
};
}

struct Carafe {
  Carafe(int a, int b);
  Carafe();
};

struct Decanter {
  Decanter(int a);
  Decanter(const char* s);
  Decanter();
};

struct Jug {
  Jug(int a, int b = 1);
  Jug();
};

struct Urn {
  Urn(int a, int b);
  Urn();
};

namespace pot {
struct Urn {
  Urn(int a, int b);
};
}

struct Flagon {
  Flagon(int a);
  Flagon();
};

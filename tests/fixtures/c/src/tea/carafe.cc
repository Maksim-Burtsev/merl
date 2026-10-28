#include "tea/carafe.hh"
#include "tea/carafe_twin.hh"

namespace pot {
Carafe::Carafe(int ml) {}
Carafe::Carafe() : Carafe(500) {}
//                 ^ d: src/tea/carafe.cc:5
Carafe::Carafe(const char* name, int ml) : Carafe(ml) {}
//                                         ^ d: !jump
Urn::Urn(int a, int b) {}
}

Carafe::Carafe() : Carafe(1, 2) {}
//                 ^ d: include/tea/carafe.hh:10
Decanter::Decanter() : Decanter(250) {}
//                     ^ d: !jump
Jug::Jug() : Jug(1) {}
//           ^ d: !jump
Urn::Urn() : Urn(1, 2) {}
//           ^ d: include/tea/carafe.hh:26
Flagon::Flagon() : Flagon(1) {}
//                 ^ d: !jump

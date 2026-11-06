#pragma once

#include "tea/trivet.hh"

namespace pantry {

class Saucer : public Trivet {
  void Spin();
  void Tilt(Stand* s);
};

}

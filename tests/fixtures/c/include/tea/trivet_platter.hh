#pragma once

#include "tea/trivet.hh"

namespace pantry {

using TrivetBase = Trivet;

class Platter : public TrivetBase {
  void Spin();
};

}

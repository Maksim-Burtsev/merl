#include "tea/dripper.hh"

namespace infuse {
class SlowDripper : public Dripper {
 public:
  int Drip(int drops) override { return drops; }
  //  ^ d: picker include/tea/dripper.hh:6
};
}  // namespace infuse

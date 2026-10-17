#include "tea/tidy.hh"
#include "tea/tidy_in.hh"

namespace infuse {
int wrap() {
  tidy::Napkin w{1};
  //     ^ d: picker include/tea/tidy.hh:4, include/tea/tidy_in.hh:5
  return w.size;
}
}  // namespace infuse

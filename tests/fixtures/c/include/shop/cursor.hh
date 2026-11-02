#pragma once

namespace shop {

class Cursor {
 public:
  virtual bool Valid() const = 0;
  //           ^ d: !jump
};

}  // namespace shop

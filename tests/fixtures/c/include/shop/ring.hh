#pragma once

namespace shop {

template <typename Key, class Order>
class Ring {
 public:
  class Cursor {
   public:
    bool Valid() const;
  };
};

template <typename Key, class Order>
inline bool Ring<Key, Order>::Cursor::Valid() const {
  return true;
}

}  // namespace shop

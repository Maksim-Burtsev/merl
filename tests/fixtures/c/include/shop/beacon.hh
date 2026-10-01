#pragma once

namespace shop {

class Beacon {
 public:
  // Ready says whether a Beacon (a Laser or
  // a Camera) can scan.
  virtual bool Ready() const = 0;
};

class Laser : public Beacon {
 public:
  bool Ready() const override { return true; }
};

class Camera : public Beacon {
 public:
  bool Ready() const override { return false; }
};

class Cabinet {
 public:
  unsigned long Count() const;
  void Fill(int items,
            int boxes);
  int Lock(int code);
};

template <typename K>
class Shelf {
 public:
  class Beacon {
   public:
    bool Ready() const;
  };
};

template <typename K>
bool Shelf<K>::Beacon::Ready() const { return true; }

namespace depot {
class Cabinet {
 public:
  int Lock(int code);
};
}  // namespace depot

}  // namespace shop

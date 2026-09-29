#pragma once
#include <string>

#define SHOP_API

namespace shop {

class Tariff {
 public:
  explicit Tariff(int base) : base_(base) {}
  int rate() const { return base_; }
  std::string describe() const;

 private:
  int base_;
};

class Coupon {
 public:
  int rate() const { return 2; }
  std::string describe() const { return "coupon"; }
};

class Priced {
 public:
  virtual int price() const = 0;
};

enum class Offer { Plain, Cut };

template <typename T>
struct Box {
  T item;
};

template <>
struct Box<int> {
  int item;
};

class SHOP_API Ledger {
 public:
  void clear() {}
};

struct __attribute__((__packed__)) Label {
  char code;
};

using Cents = long;

}  // namespace shop

// Every construct `f` folds in C++, and the cases that once broke it.
#include <map>  // f: 2-4
#include <string>  // f: 2-4
#include <vector>  // f: 2-4

namespace shop {  // f: 6-86

/*  // f: 8-10
 * A comment over lines folds.  // f: 8-10
 */  // f: 8-10
enum class Color : int {  // f: 11-14
  kRed,  // f: 11-14
  kGreen,  // f: 11-14
};  // f: 11-14

template <typename T>  // f: 16-32
class Box : public Base<T> {  // f: 17-32
 public:  // f: 17-32
  explicit Box(T value)  // f: 19-23
      : value_(value),  // f: 19-23
        count_{0} {  // f: 19-23
    Touch();  // f: 19-23
  }  // f: 19-23

  T Get() const override {  // f: 25-27
    return value_;  // f: 25-27
  }  // f: 25-27

 private:  // f: 17-32
  T value_;  // f: 17-32
  int count_;  // f: 17-32
};  // f: 17-32

struct Point {  // f: 34-37
  int x;  // f: 34-37
  int y;  // f: 34-37
};  // f: 34-37

int Sum(const std::vector<int>& values) {  // f: 39-84
  int total = 0;  // f: 39-84
  for (int v : values) {  // f: 41-43
    total += v;  // f: 39-84
  }  // f: 39-84
  auto add = [&total](int v) {  // f: 44-46
    total += v;  // f: 44-46
  };  // f: 44-46
  std::map<int, std::string> names = {  // f: 47-50
      {1, "one"},  // f: 39-84
      {2, "two"},  // f: 39-84
  };  // f: 39-84
  Point p{  // f: 51-54
      1,  // f: 39-84
      2,  // f: 39-84
  };  // f: 39-84
  const char* raw = R"json(  // f: 39-84
    { "not": "a block" }  // f: 39-84
  )json";  // f: 39-84
  try {  // f: 58-62
    add(p.x);  // f: 39-84
  } catch (const std::exception& e) {  // f: 60-62
    total = -1;  // f: 39-84
  }  // f: 39-84
  switch (total) {  // f: 63-73
    case 0:  // f: 64-66
      total = 1;  // f: 39-84
      break;  // f: 39-84
    case 1: {  // f: 67-70
      total = 2;  // f: 39-84
      break;  // f: 39-84
    }  // f: 39-84
    default:  // f: 71-72
      break;  // f: 39-84
  }  // f: 39-84
#ifdef DEBUG  // f: 74-79
  if (total > 10)  // f: 75-78
    total = 10;  // f: 39-84
  else  // f: 39-84
    total = 0;  // f: 39-84
#endif  // f: 39-84
  do {  // f: 80-82
    total--;  // f: 39-84
  } while (total > 0);  // f: 39-84
  return total + raw[0];  // f: 39-84
}  // f: 39-84

}  // namespace shop  // f: 6-86

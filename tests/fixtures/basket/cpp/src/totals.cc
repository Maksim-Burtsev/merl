#include <string>
#include <vector>

std::vector<int> doubled(const std::vector<int>& prices) {
  std::vector<int> out;
  auto seen = std::vector<int>();
  for (int p : prices) {
    out.push_back(p * 2);
    //  ^ d: none
    seen.push_back(p);
    //   ^ d: none
  }
  std::string text = "a,b";
  return text.empty() ? out : seen;
  //          ^ d: none
}

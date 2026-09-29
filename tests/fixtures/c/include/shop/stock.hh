#pragma once

namespace shop {
void stock_note(int n);
inline void stock_note(double n) {}
int stock_tag(const char *name, int n = 0);
}  // namespace shop

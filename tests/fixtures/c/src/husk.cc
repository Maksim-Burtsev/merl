// int husk_free(int n) { return n; }
// struct HuskCup { int husk_size; };
const char *husk_sample = "int husk_sampled(void) { return 0; }";

int husk_size_of(int n) { return husk_free(n) + husk_sampled(); }
//                               ^ d: none
//                                              ^ d: none

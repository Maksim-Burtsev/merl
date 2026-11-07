// int husk_line(void) { return 0; }
/* int husk_block(void) {
   } */
    // int husk_indent(void) { return 0; }
// int husk_real(void) { return 0; }

int husk_real(void) { return 1; }

int husk_use(void) {
    return husk_line() + husk_block() + husk_indent() + husk_real();
//         ^ d: none
//                       ^ d: none
//                                      ^ d: none
//                                                      ^ d: src/husk.c:7
}

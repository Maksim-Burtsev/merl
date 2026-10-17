int pour_c(int cups);

int pour_c(int cups) { return cups; }

int serve_c(void) { return pour_c(6); }
//                         ^ d: src/brew_line.c:3

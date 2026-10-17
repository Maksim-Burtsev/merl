int pour(int cups);

int pour(int cups) { return cups; }

int serve() { return pour(6); }
//                   ^ d: src/tea/brew_line.cc:3

int twice(int pour) { return pour + pour; }
//                           ^ d: src/tea/brew_line.cc:8

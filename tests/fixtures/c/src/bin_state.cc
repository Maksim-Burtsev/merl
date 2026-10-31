int Weigh(void* arg) {
  struct State {
    int grams;

    static bool Match(void* arg) {
      State* state = static_cast<State*>(arg);
//    ^ d: src/bin_state.cc:2
      return state->grams > 0;
    }
  };
  State s{1};
//^ d: src/bin_state.cc:2
  return State::Match(&s);
}

int Count(void* arg) {
  struct State {
    int items;
  };
  auto tally = [](void* p) {
    struct State {
      int left;
    };
    State* s = static_cast<State*>(p);
//  ^ d: src/bin_state.cc:21
    return s->left;
  };
  State* state = static_cast<State*>(arg);
//^ d: src/bin_state.cc:17
  return state->items + tally(arg);
}

int Sort(void* arg) {
  State* state = static_cast<State*>(arg);
//^ d: !jump
  return 0;
}

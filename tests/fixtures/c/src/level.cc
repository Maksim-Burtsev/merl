class Compaction {
 public:
  int level() const { return level_; }
  int level_;
};
struct Manual {
  int level;
};
int manual_level(Manual* m) { return m->level; }
//                                      ^ d: src/level.cc:7
int compaction_level(Compaction* c) { return c->level(); }
//                                              ^ d: src/level.cc:3

#define LEVEL_CAP 7 /* The deepest level — a comment that runs on
                       past its directive { with a brace } */
template <typename Key>
class Tower {
 public:
  explicit Tower(int h);
  int height_;
};

template <typename Key>
Tower<Key>::Tower(int h)
    : height_(h) {}
    //^ d: src/level.cc:20
int tower_height(Tower<int>* t) { return t->height_; }
//                                          ^ d: src/level.cc:20

template <typename Key, class Comparator>
class Ladder {
 public:
  struct Rung;
  class Iterator {
   public:
    void Seek(const Key& target);
  };
};

template <typename Key, class Comparator>
struct Ladder<Key, Comparator>::Rung {
//     ^ d: picker src/ladder.cc:2, src/ladder.cc:23
  Key key;
};

template <typename Key, class Comparator>
inline void Ladder<Key, Comparator>::Iterator::Seek(const Key& target) {
//          ^ d: picker src/ladder.cc:2, src/ladder.cc:23
}

template <>
class Ladder<int, int> {
 public:
  int top;
};

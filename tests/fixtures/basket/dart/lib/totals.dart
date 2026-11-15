List<int> doubled(List<int> prices) {
  final out = <int>[];
  var seen = [];
  for (final p in prices) {
    out.add(p * 2);
    //  ^ d: none
    seen.add(p);
    //   ^ d: none
  }
  final text = "a,b";
  return text.contains("a") ? out : out;
  //          ^ d: none
}

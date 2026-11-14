class Crock {}

class Vat<T> {}

extension Rim
    on Crock {
  void rim() {}
}

extension AnyRim on Object {
  void rim() {}
}

extension Hem<T extends Comparable<T>> on Vat<T> {
  void hem() {}
}

extension AnyHem on Object {
  void hem() {}
}

extension<T extends Comparable<T>> on Vat<T> {
  void lip() {}
}

extension AnyLip on Object {
  void lip() {}
}

void lids(Crock crock, Vat<int> vat) {
  crock.rim();
  //    ^ d: !jump
  vat.hem();
  //  ^ d: !jump
  vat.lip();
  //  ^ d: !jump
}

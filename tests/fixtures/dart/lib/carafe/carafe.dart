class Carafe {
  void boil() {}

  static CarafeBuilder builder() => CarafeBuilder();
}

class CarafeBuilder {
  CarafeBuilder size(int n) => this;

  Carafe finish() => Carafe();
}

class Wok {
  void boil() {}

  Wok size(int n) => this;

  Wok finish() => this;
}

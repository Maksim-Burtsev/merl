class Coupon {
  Coupon(this.rate);

  final double rate;

  String describe() => 'coupon $rate';

  static Coupon parse(String s) => Coupon(double.parse(s));
}

double discount(double total) => total * 0.9;

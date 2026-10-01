class Coupon {
  Coupon(this.rate);

  final double rate;

  String describe() => 'coupon $rate';
}

double discount(double total) => total * 0.9;

/// Every declaration form `d` reads in Dart (#414).
library;

typedef Json = Map<String, dynamic>;
typedef void Callback(int code);

abstract interface class Repo {
  Future<Tariff> find(int id);
}

sealed class Result {}

base class Animal {}

final class Money {}

mixin class Walker {}

mixin Trotter on Animal {}

extension StringX on String {
  String get shout => toUpperCase();
}

extension on int {
  int get twice => this * 2;
}

extension type UserId(int id) {}

enum Status { open, closed }

enum Level {
  low,
  high('h');

  const Level([this.code = '']);
  final String code;
}

final cache = <String, Tariff>{};

class Tariff {
  const Tariff({required this.rate});
  factory Tariff.fromJson(Json json) => Tariff(rate: json['rate'] as double);
  Tariff.flat() : rate = 1;
  Tariff.empty();

  final double rate;
  static const int limit = 5;
  late final Repo repo;
  String? label;
  int count = 0;

  String describe() => 'tariff $rate';
  static Tariff parse(String s) => Tariff(rate: double.parse(s));
  String get title => 'tariff';
  set title(String value) {}
  double discount(double total) => total * rate;
  String weigh(int grams) => '$grams g';
  build(context) {}
}

T first<T>(List<T> items) => items.first;

main() {}

Tariff _$TariffFromJson(Json json) => Tariff(rate: 1);

class $TariffCopyWith {}

const help = '''
class Ghost {
  void haunt() {}
''';

/*
class Phantom {
*/

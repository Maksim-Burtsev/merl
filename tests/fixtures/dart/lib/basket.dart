import 'package:shop/money.dart';
import 'coupon.dart' as c;
import 'courier.dart' show weigh;
import 'pricing.dart';
import 'pricing.dart' show Tariff;
import 'courier.dart' as k;

/// The basket probes `d` on every form of `lib/pricing.dart` (#414).
class Basket {
  Basket(this.tariff);

  final Tariff tariff;

  String total(int cents) {
    final price = formatPrice(cents);
    //            ^ d: lib/money.dart:1
    // status: formatPrice: by name, 1 match
    final cut = c.discount(42) + tariff.discount(1);
    //            ^ d: lib/coupon.dart:11
    // status: via import coupon.dart
    final coupon = c.Coupon(0.5);
    //               ^ d: lib/coupon.dart:1
    // status: via import coupon.dart
    final parsed = c.Coupon.parse('1');
    //                      ^ d: lib/coupon.dart:8
    // status: via import coupon.dart
    // `courier.dart` only hands `first` on: the search by name answers.
    k.first([1]);
    //^ d: lib/pricing.dart:64
    // status: by name
    // A member hanging off a call is no name an import binds.
    Tariff.parse('1').weigh(500);
    //                ^ d: !jump
    final kg = weigh(500);
    //         ^ d: lib/courier.dart:3
    // status: via import courier.dart
    print([Json, Callback, Repo]);
    //     ^ d: lib/pricing.dart:4
    //           ^ d: lib/pricing.dart:5
    //                     ^ d: lib/pricing.dart:7
    tariff.repo.find(1);
    //          ^ d: lib/pricing.dart:8
    print([Result, Animal, Money, Walker]);
    //     ^ d: lib/pricing.dart:11
    //             ^ d: lib/pricing.dart:13
    //                     ^ d: lib/pricing.dart:15
    //                            ^ d: lib/pricing.dart:17
    print([Trotter, UserId]);
    //     ^ d: lib/pricing.dart:19
    //              ^ d: lib/pricing.dart:29
    'x'.shout; StringX('x');
    //  ^ d: lib/pricing.dart:22
    //         ^ d: lib/pricing.dart:21
    final s = Status.open; final l = Level.high;
    //        ^ d: lib/pricing.dart:31
    //               ^ d: lib/pricing.dart:31
    //                                     ^ d: lib/pricing.dart:35
    print([l.code, cache]);
    //       ^ d: lib/pricing.dart:38
    //             ^ d: lib/pricing.dart:41
    const Tariff(rate: 1); Tariff.fromJson({}); Tariff.flat();
    //    ^ d: picker lib/pricing.dart:43, lib/pricing.dart:44
    //                            ^ d: lib/pricing.dart:45
    //                                                 ^ d: lib/pricing.dart:46
    print([Tariff.limit, tariff.repo, tariff.label, tariff.count]);
    //     ^ d: lib/pricing.dart:43
    //            ^ d: lib/pricing.dart:50
    //                          ^ d: lib/pricing.dart:51
    //                                       ^ d: lib/pricing.dart:52
    //                                                     ^ d: lib/pricing.dart:53
    print([tariff.describe(), Tariff.parse('1'), tariff.title]);
    //            ^ d: picker lib/pricing.dart:55, lib/coupon.dart:6
    //                               ^ d: lib/pricing.dart:56
    // status: via import pricing.dart
    //                                                  ^ d: picker lib/pricing.dart:57, lib/pricing.dart:58
    print([first([1]), main(), _$TariffFromJson({}), $TariffCopyWith()]);
    //     ^ d: lib/pricing.dart:64
    //                 ^ d: lib/pricing.dart:66
    //                         ^ d: lib/pricing.dart:68
    //                                               ^ d: lib/pricing.dart:70
    // A call, a constructor's call written as a statement, a method with no type: no rule.
    Navigator.push(context, route);
    //        ^ d: none
    if (ok) setState(() {
    //      ^ d: none
    });
    Tariff.empty();
    //     ^ d: none
    tariff.build(context);
    //     ^ d: none
    // What a `'''` string and a `/* */` comment hold declares nothing.
    print([Ghost(), Phantom(), haunt()]);
    //     ^ d: none
    //              ^ d: none
    //                         ^ d: none
    return '$price $cut $kg';
  }
}

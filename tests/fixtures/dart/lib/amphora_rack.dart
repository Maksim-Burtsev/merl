import 'amphora.dart';

class AmphoraRack<Amphora> {
//                ^ d: lib/amphora_rack.dart:3
  Amphora held;
//^ d: lib/amphora_rack.dart:3
  AmphoraRack(this.held);
  Amphora take(Amphora x) => x;
//^ d: lib/amphora_rack.dart:3
  //           ^ d: lib/amphora_rack.dart:3
  Amphora own<Amphora>(Amphora x) => x;
//^ d: lib/amphora_rack.dart:11
  //          ^ d: lib/amphora_rack.dart:11
  //                   ^ d: lib/amphora_rack.dart:11
}

class AmphoraPlain {
  Amphora plain(Amphora x) => x;
//^ d: lib/amphora.dart:1
  //            ^ d: lib/amphora.dart:1
}

class AmphoraBound<T extends Amphora> {
//                 ^ d: lib/amphora_rack.dart:23
//                           ^ d: lib/amphora.dart:1
  T? held;
//^ d: lib/amphora_rack.dart:23
}

abstract class AmphoraHolder<Amphora> {
//                           ^ d: lib/amphora_rack.dart:30
  Amphora get();
//^ d: lib/amphora_rack.dart:30
}

typedef AmphoraPair<Amphora> = List<Amphora>;
//                  ^ d: lib/amphora_rack.dart:36
//                                  ^ d: lib/amphora_rack.dart:36

class AmphoraWrapped<
  Amphora
//^ d: lib/amphora_rack.dart:41
> {
  Amphora? held;
//^ d: lib/amphora_rack.dart:41
}

class AmphoraWrappedFns {
  Amphora wrapped<Amphora>(
//^ d: lib/amphora_rack.dart:49
    Amphora x,
  //^ d: lib/amphora_rack.dart:49
    int y,
  ) => x;
  Amphora plain(
//^ d: lib/amphora.dart:1
    Amphora x,
  //^ d: lib/amphora.dart:1
  ) => x;
}

class AmphoraCrossBound<
  T extends Amphora,
  Amphora
> {
  Amphora? held;
//^ d: lib/amphora_rack.dart:64
}

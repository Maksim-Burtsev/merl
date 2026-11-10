import 'package:steep_annotation/steep_annotation.dart';
import 'rigs/routes.dart';

// #1452: `steep_annotation` is not in the pub cache, so `JsonKey` is the import's, never the
// namesake in `rigs/json.dart`, which this file does not import; nor are `override` and
// `deprecated` of dart:core the functions there.
@Route('/home', auth: true)
//^ d: lib/tea/rigs/routes.dart:1
class Leaf {
  @JsonKey(name: 'id')
  //^ d: lib/tea/keyed.dart:1
  //status: via import steep_annotation (not installed)
  //         ^ d: none
  final int id = 0;

  @override
  //^ d: none
  String toString() => '';

  @deprecated
  //^ d: none
  int old() => const JsonKey(name: 'x').hashCode;
  //                 ^ d: lib/tea/keyed.dart:1
}

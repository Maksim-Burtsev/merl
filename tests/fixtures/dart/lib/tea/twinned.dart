import 'package:steep_annotation/steep_annotation.dart';
import 'package:steep_codec/steep_codec.dart';
import 'package:steep_shown/steep_shown.dart' show Codec;
import 'package:steep_prefixed/steep_prefixed.dart' as prefixed;

// #1452: two packages not in the pub cache could give `JsonKey`, so `d` offers both imports,
// never the namesake in `rigs/json.dart`, which this file does not import. An import whose
// `show` leaves the name out, or one under a prefix, cannot give it.
class Twin {
  @JsonKey(name: 'id')
  //^ d: picker lib/tea/twinned.dart:1, lib/tea/twinned.dart:2
  final int id = 0;

  @prefixed.JsonKey(name: 'id')
  //        ^ d: !jump
  final int other = 0;
}

import 'brewer.dart';
import 'pot.dart';

int serve() {
  final mugful = 5;
  final pot = Kwpot();
  kwbrew(mugful: mugful);
  //     ^ d: picker lib/kwcafe/uses.dart:5, lib/kwcafe/uses.dart:16, lib/kwcafe/brewer.dart:1, lib/kwcafe/pot.dart:2; want lib/kwcafe/brewer.dart:3 (#1446)
  //             ^ d: lib/kwcafe/uses.dart:5
  pot.kwbrew(mugful: 4);
  //         ^ d: picker lib/kwcafe/uses.dart:5, lib/kwcafe/uses.dart:16, lib/kwcafe/brewer.dart:1, lib/kwcafe/pot.dart:2; want lib/kwcafe/pot.dart:4 (#1446)
  return mugful;
}

int neighbours(bool flag) {
  final mugful = 1;
  final r = (mugful: 1, b: 2);
  //         ^ d: !jump
  final m = {mugful: 1};
  //         ^ d: lib/kwcafe/uses.dart:16
  kwbrew(mugful: flag ? mugful : 2);
  //                    ^ d: lib/kwcafe/uses.dart:16
  return r.mugful;
}

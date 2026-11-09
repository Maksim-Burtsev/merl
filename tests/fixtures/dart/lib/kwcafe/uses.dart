import 'brewer.dart';
import 'pot.dart';

int serve() {
  final mugful = 5;
  final pot = Kwpot();
  kwbrew(mugful: mugful);
  //     ^ d: lib/kwcafe/brewer.dart:3
  //             ^ d: lib/kwcafe/uses.dart:5
  pot.kwbrew(mugful: 4);
  //         ^ d: lib/kwcafe/pot.dart:4
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

int pours() {
  final mugful = 6;
  kwpour(1, size: 2, mugful: mugful);
  //                 ^ d: lib/kwcafe/brewer.dart:5
  final cup = Kwcup.sized(
    size: 1,
    mugful: mugful,
  //^ d: lib/kwcafe/pot.dart:13
  );
  return cup.size;
}

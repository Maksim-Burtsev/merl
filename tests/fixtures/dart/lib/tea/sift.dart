import 'sieve.dart' show ifEmpty;

// A `show` name starting with `if` is no conditional import.
int sift() => ifEmpty(1);
//            ^ d: lib/tea/sieve.dart:1

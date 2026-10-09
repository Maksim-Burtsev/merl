// Every construct `f` folds in Dart, and the cases that once broke it.
import 'dart:async';
import 'package:a/b.dart'
    show C;  // f: 2-4

export 'src/c.dart';  // f: none
part 'd.dart';  // f: none

@Immutable('x')  // f: 9-71
@a.Meta.named()  // f: 9-71
abstract class A extends B with M {  // f: 9-71
  A({  // f: 9-71
    required this.x,  // f: 9-71
  }) : super(  // f: 14-16
         x,  // f: 9-71
       );  // f: 9-71

  final int x;  // f: 9-71

  int get y =>  // f: 20-21
      x + 1;  // f: 20-21

  @override  // f: 9-71
  bool operator ==(Object other) =>  // f: 24-26
      other is A &&  // f: 24-26
      other.x == x;  // f: 24-26

  Selectable<({String key, int? value})> rows() =>  // f: 28-29
      db.select(t).map((r) => (key: r.key, value: r.value));  // f: 28-29

  Future<void> run(int n) async {  // f: 31-63
    if (n > 0) {  // f: 32-38
      await go(  // f: 33-38
        n,  // f: 31-63
        const [  // f: 35-37
          1,  // f: 31-63
        ],  // f: 31-63
      );  // f: 31-63
    } else {  // f: 39-46
      final m = {  // f: 40-42
        'a': 1,  // f: 31-63
      };  // f: 31-63
      print('${m.map((k, v) {
        return k;
      })}');
    }  // f: 31-63
    final s = '''
multi ${'}'}
''';
    final t = 'a'  // f: 50-51
        'b';  // f: 31-63
    switch (n) {  // f: 52-55
      case 1:  // f: 31-63
        break;  // f: 31-63
    }  // f: 31-63
    items.forEach((item) {  // f: 56-58
      print(item);  // f: 56-58
    });  // f: 56-58
    final f = (int x) =>  // f: 31-63
        x * 2;  // f: 31-63
    final i = list[  // f: 31-63
        0];  // f: 31-63
  }  // f: 31-63

  Widget build(BuildContext context) => switch (state) {  // f: 65-70
    S.a => const Text(  // f: 66-68
      'a',  // f: 65-70
    ),  // f: 65-70
    _ => const SizedBox(),  // f: 65-70
  };  // f: 65-70
}  // f: 9-71

mixin M on B {  // f: none
  void f() {  // f: 74-76
    g();  // f: 74-76
  }  // f: 74-76
}  // f: none

enum E {  // f: 79-87
  a(  // f: 80-82
    1,  // f: 79-87
  ),  // f: 79-87
  b(2);  // f: 79-87

  const E(this.v);  // f: 79-87
  final int v;  // f: 79-87
}  // f: 79-87

extension X on int {  // f: 89-93
  int twice() {  // f: 90-92
    return this * 2;  // f: 90-92
  }  // f: 90-92
}  // f: 89-93

/* a /* nested */ comment {
   with a brace */
void main() => run(  // f: 97-99
      1,  // f: 97-99
    );  // f: 97-99

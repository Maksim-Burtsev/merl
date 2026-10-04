// Every construct `f` folds in Java, and the cases that once broke it.
package a;

import java.util.List;
// a comment ends a run of imports
import java.util.Map;  // f: 6-8

import static x.Y.z;  // f: 6-8

@Foo(  // f: 10-12
    a = 1  // f: 10-12
)  // f: 10-12
public class A<T> extends B {  // f: 13-75
    @Inject  // f: 14-21
    public A(  // f: 14-21
        int x  // f: 14-21
    ) throws X {  // f: 14-21
        super(  // f: 18-20
            x  // f: 14-21
        );  // f: 14-21
    }  // f: 14-21

    interface I {  // f: 13-75
        void f();  // f: 13-75
    }  // f: 13-75

    enum E {  // f: 13-75
        X(  // f: 28-30
            1  // f: 13-75
        ),  // f: 13-75
        Y {  // f: 31-34
            void g() {  // f: 32-33
            }  // f: 32-33
        };  // f: 31-34
    }  // f: 13-75

    record R(int a) {  // f: 37-40
        R {  // f: 38-39
        }  // f: 38-39
    }  // f: 37-40

    static {  // f: 42-51
        int[] a = {  // f: 13-75
            1,  // f: 13-75
        };  // f: 13-75
        String s = """  // f: 13-75
            { not a block  // f: 13-75
            """;  // f: 13-75
        char c = '{';  // f: 13-75
        // }  // f: 13-75
    }  // f: 13-75

    List<String> f(int x) {  // f: 53-74
        switch (x) {  // f: 53-74
            case 1 -> {  // f: 55-57
                foo();  // f: 55-57
            }  // f: 55-57
            default -> {  // f: 58-61
                bar(  // f: 59-60
                    1);  // f: 58-61
            }  // f: 58-61
        }  // f: 53-74
        Runnable r = () -> {  // f: 63-65
            go();  // f: 63-65
        };  // f: 63-65
        Object o = new Object() {  // f: 66-70
            int h() {  // f: 67-69
                return 1;  // f: 67-69
            }  // f: 67-69
        };  // f: 66-70
        return List.of(  // f: 71-73
            "a"  // f: 53-74
        );  // f: 53-74
    }  // f: 53-74
}  // f: 13-75

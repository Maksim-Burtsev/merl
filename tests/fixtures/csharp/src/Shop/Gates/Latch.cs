using Shop.Gates.Kit;

namespace Shop.Larder.Inner;

public class Latch
{
    public object Hang() => new Hinge();
    //                          ^ d: src/Shop/Gates/Kit.cs:3

    public object Clasp() => new Hasp();
    //                           ^ d: src/Shop/Gates/Kit.cs:10
}

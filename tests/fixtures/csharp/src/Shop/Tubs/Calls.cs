using System;
using System.Collections.Generic;

namespace Shop.Tubs;

public class Calls
{
    private List<Tub> tubs = new();
    private Tun t;

    void Each(Action<Tub> f) { }
    bool Test(Func<Tub, bool> f) => true;
    void Two(Action<Tub> f) { }
    void Two(Action<Tun> f) { }
    void Gen<T>(Action<T> f) { }
    void Pair(Action<Tub> f, int n) { }

    void A()
    {
        Each(t =>
        {
            t.Dunk();
            //^ d: src/Shop/Tubs/Tub.cs:5
        });
    }
    void B()
    {
        Each(t => {
            t.Dunk();
            //^ d: src/Shop/Tubs/Tub.cs:5
        });
    }
    void C()
    {
        Each((t) =>
        {
            t.Dunk();
            //^ d: src/Shop/Tubs/Tub.cs:5
        });
    }
    void D() => Each(t => t.Dunk());
    //                      ^ d: src/Shop/Tubs/Tub.cs:5
    void E() => Test(t => t.Dunk());
    //                      ^ d: src/Shop/Tubs/Tub.cs:5
    void F() => Two(t => t.Dunk());
    //                     ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
    void G() => Gen<Tun>(t => t.Dunk());
    //                          ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
    void H() => Pair(t => t.Dunk(), 1);
    //                      ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
    void I(Action<Action<Tun>> Each) => Each(t => t.Dunk());
    //                                              ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
    void J()
    {
        void Each(Action<Tun> f) { }
        Each(t => t.Dunk());
        //          ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
    }
    void K()
    {
        tubs.ForEach(t =>
        {
            t.Dunk();
            //^ d: src/Shop/Tubs/Tub.cs:5
        });
    }
    void L() => t.Dunk();
    //            ^ d: src/Shop/Tubs/Tun.cs:5

    class Inner
    {
        void M() => Each(t => t.Dunk());
        //                      ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
    }
    class Own
    {
        void Each(Action<Tun> f) { }
        void N() => Each(t => t.Dunk());
        //                      ^ d: src/Shop/Tubs/Tun.cs:5
    }
}

public class Based : Calls
{
    void O() => Each(t => t.Dunk());
    //                      ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
}

public class Boxed<T>
{
    void Each(Action<T> f) { }
    void P() => Each(t => t.Dunk());
    //                      ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
    void Each2(Action<Tub> f) { }
    void Q() { Each2(t => t.Dunk()); Each(t => t.Dunk()); }
    //                      ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
    //                                           ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5
    void R() => Each2(t => new Each2(u => t.Dunk()));
    //                                      ^ d: src/Shop/Tubs/Tub.cs:5
}

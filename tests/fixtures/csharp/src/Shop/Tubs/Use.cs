using System;
using System.Collections.Generic;
using System.Linq;

namespace Shop.Tubs;

public class Use
{
    private List<Tub> tubs = new();
    private IEnumerable<Tub> seq;
    private Tun c;

    void A() => tubs.ForEach(x => x.Dunk());
    //                              ^ d: src/Shop/Tubs/Tub.cs:5
    void B() => tubs.Select(x => x.Dunk()).ToList();
    //                             ^ d: src/Shop/Tubs/Tub.cs:5
    void C() => seq.Where(x => x.Dunk());
    //                           ^ d: src/Shop/Tubs/Tub.cs:5
    void D(List<Tub> more) => more.Where(c => c.Dunk());
    //                                          ^ d: src/Shop/Tubs/Tub.cs:5
    void E(IEnumerable<Tub> more) => more.OrderBy(x => x.Slop());
    //                                                   ^ d: src/Shop/Tubs/Tub.cs:6
    void F()
    {
        var local = new List<Tub>();
        local.ForEach(x => x.Dunk());
        //                   ^ d: src/Shop/Tubs/Tub.cs:5
    }
    void G()
    {
        List<Tub> local = tubs;
        local.Any(x => x.Dunk());
        //               ^ d: src/Shop/Tubs/Tub.cs:5
    }
    void H() => tubs.Sort((a, b) => a.Slop() - b.Slop());
    //                                ^ d: src/Shop/Tubs/Tub.cs:6
    //                                           ^ d: src/Shop/Tubs/Tub.cs:6
    void Each(Action<Tub> f) { }
    void I() => Each(x => x.Dunk());
    //                      ^ d: picker src/Shop/Tubs/Hogshead.cs:5, src/Shop/Tubs/Tub.cs:5, src/Shop/Tubs/Tun.cs:5; want src/Shop/Tubs/Tub.cs:5 (#1488)
    void J() { Action<Tub> f = x => x.Dunk(); }
    //                                ^ d: src/Shop/Tubs/Tub.cs:5
    void K() => c.Dunk();
    //            ^ d: src/Shop/Tubs/Tun.cs:5
    void L() => seq.First(x => x.Dunk());
    //                           ^ d: src/Shop/Tubs/Tub.cs:5
    void M() { tubs.ForEach(c => c.Dunk()); }
    //                             ^ d: src/Shop/Tubs/Tub.cs:5
    void N() => tubs.Where(x => x.Dunk()).Select(y => y.Slop());
    //                            ^ d: src/Shop/Tubs/Tub.cs:5
    //                                                  ^ d: src/Shop/Tubs/Tub.cs:6
    void O() => this.tubs.ForEach(x => x.Dunk());
    //                                   ^ d: src/Shop/Tubs/Tub.cs:5
    void P(List<Tun> tuns) => tuns.Sort((a, b) => a.Slop() - b.Slop());
    //                                                         ^ d: src/Shop/Tubs/Tun.cs:6
    void Q() => tubs.Select((x, i) => x.Dunk());
    //                                  ^ d: !jump
}

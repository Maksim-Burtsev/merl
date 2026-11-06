using System;
using System.Collections.Generic;
using System.Linq;

namespace Shop.Rebound;

public class Shelf
{
    Flask x = new Flask();

    void Lambda(Flask k)
    {
        Func<Ewer, int> a = k => k.Decant(); k.Decant();
        //                         ^ d: src/Shop/Rebound/Ewer.cs:5
        //                                     ^ d: picker src/Shop/Rebound/Flask.cs:5, …
    }

    void Delegate(Flask k)
    {
        Func<Ewer, int> b = delegate (Ewer k) { return k.Decant(); };
        //                                               ^ d: picker src/Shop/Rebound/Ewer.cs:5, …
    }

    void Foreach(List<Ewer> es)
    {
        foreach (var x in es) x.Decant(); x.Decant();
        //                      ^ d: src/Shop/Rebound/Ewer.cs:5
        //                                  ^ d: picker src/Shop/Rebound/Flask.cs:5, …
    }

    void Catch()
    {
        try { } catch (Spill x) { x.Decant(); } x.Decant();
        //                          ^ d: src/Shop/Rebound/Spill.cs:5
        //                                        ^ d: picker src/Shop/Rebound/Flask.cs:5, …
    }

    void Using()
    {
        { using var x = new Ewer(); x.Decant(); } x.Decant();
        //                            ^ d: picker src/Shop/Rebound/Ewer.cs:5, …
        //                                          ^ d: src/Shop/Rebound/Flask.cs:5
    }

    void Block()
    {
        { var x = new Ewer(); x.Decant(); } x.Decant();
        //                      ^ d: picker src/Shop/Rebound/Ewer.cs:5, …
        //                                    ^ d: src/Shop/Rebound/Flask.cs:5
    }

    void Property(object o)
    {
        if (o is Ewer { } x) x.Decant();
        //                     ^ d: picker src/Shop/Rebound/Ewer.cs:5, …
    }

    int Arms(object o)
    {
        return o switch { Ewer x => x.Decant(), Spill x => x.Decant(), _ => x.Decant() };
        //                            ^ d: src/Shop/Rebound/Ewer.cs:5
        //                                                   ^ d: picker src/Shop/Rebound/Spill.cs:5, …
        //                                                                    ^ d: picker src/Shop/Rebound/Flask.cs:5, …
    }

    void Query(List<Ewer> es)
    {
        var q = es.Select(e => e).Where(x => true).Count(); var r = from x in es select x.Decant();
        //                                                                                ^ d: picker src/Shop/Rebound/Ewer.cs:5, …
    }

    void Negated(object o)
    {
        if (o is not Ewer x) return;
        x.Decant();
        //^ d: picker src/Shop/Rebound/Ewer.cs:5, …
    }

    void NegatedBlock(object o)
    {
        if (!(o is Ewer x))
        {
            throw new Exception();
        }
        x.Decant();
        //^ d: src/Shop/Rebound/Ewer.cs:5
    }

    int ArmsLines(object o)
    {
        return o switch
        {
            Spill x when x.Decant() > 0 => x.Decant(),
            //             ^ d: picker src/Shop/Rebound/Spill.cs:5, …
            //                               ^ d: picker src/Shop/Rebound/Spill.cs:5, …
            _ => x.Decant(),
            //     ^ d: src/Shop/Rebound/Flask.cs:5
        };
    }

    void QueryLines(List<Ewer> es)
    {
        var q = from x in es
                where x.Decant() > 0
                //      ^ d: picker src/Shop/Rebound/Ewer.cs:5, …
                select x;
    }
}

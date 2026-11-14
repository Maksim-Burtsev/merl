namespace Shop;

public class Stylus
{
    public string Blot() => "stylus";
}

public class Nib
{
    public string Blot() => "nib";
}

public class Quoted
{
    public string A(Stylus s) => "s.Blot()";
    //                              ^ d: none
    public string B(Stylus s) => $"s.Blot() {s.Blot()} {{s.Blot()}}";
    //                               ^ d: none
    //                                         ^ d: src/Shop/Quoted.cs:5
    //                                                     ^ d: none
    public string C(Stylus s) => @"C:\" + s.Blot() + $@"{s.Blot()} ""s.Blot()""";
    //                                      ^ d: src/Shop/Quoted.cs:5
    //                                                     ^ d: src/Shop/Quoted.cs:5
    //                                                                   ^ d: none
    public string D(Stylus s) => """
        s.Blot() "
    //    ^ d: none
    """;
    public string E(Stylus s) => "a".Replace(@"""", "'") + s.Blot();
    //                                                       ^ d: src/Shop/Quoted.cs:5
    public string F(Stylus s) => $$"""{s.Blot()} {{s.Blot()}}""";
    //                                   ^ d: none
    //                                                ^ d: src/Shop/Quoted.cs:5
}

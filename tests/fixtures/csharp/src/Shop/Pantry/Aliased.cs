using System.Collections.Generic;
using Jar = Shop.Pantry.Old.Jar;

public static class Aliased
{
    public static Jar First(List<Jar> jars) => jars[0];
    //                           ^ d: src/Shop/Pantry/Old/Jar.cs:3
    //            ^ d: src/Shop/Pantry/Old/Jar.cs:3
}

public class Boxed<Jar>
{
    public Jar Item { get; set; }
    //     ^ d: src/Shop/Pantry/Aliased.cs:11
}

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
    //     ^ d: picker src/Shop/Pantry/Aliased.cs:2, src/Shop/Pantry/Jar.cs:3, src/Shop/Pantry/Old/Jar.cs:3, src/Shop/Pantry/Shadowed.cs:1, src/Shop/Tea/Infuse/Checks.cs:2
}

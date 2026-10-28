using Jar = Shop.Pantry.Old.Jar;

namespace Shop.Pantry;

public class Shadowed
{
    public Jar Pair() => new();
    //     ^ d: picker src/Shop/Pantry/Shadowed.cs:1, src/Shop/Pantry/Aliased.cs:2, src/Shop/Pantry/Jar.cs:3, src/Shop/Pantry/Old/Jar.cs:3, src/Shop/Tea/Infuse/Checks.cs:2
}

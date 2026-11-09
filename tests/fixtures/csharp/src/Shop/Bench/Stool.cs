/*
using Shop.Lab;
using static Shop.Lab.Beaker;
*/
namespace Shop.Bench;

public class Stool
{
    public int Sit()
    {
        var b = new Beaker();
        //          ^ d: src/Shop/Pricing/Pricing.cs:69
        return b.Pour();
        //       ^ d: picker src/Shop/Pricing/Pricing.cs:71, src/Shop/Lab/Beaker.cs:5, …
    }
}

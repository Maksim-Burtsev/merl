using Shop.Pricing;
using Shop.Warehouse;

namespace Shop;

public class Startup
{
    public void Run(Dictionary<string, string> claims)
    {
        claims.Remove("sub");
        //     ^ d: none
    }

    public async Task Wait(Animation animation)
    {
        await Task.Delay(animation.Delay);
        //         ^ d: none
        //                         ^ d: src/Shop/Warehouse/Courier.cs:45
    }

    public void Configure()
    {
        var courier = new Courier("depot");
        var item = courier.Name;
        var priced = new Coupon();
    }
}

public sealed class Coupons : IPriced
{
    public int Rate() => 3;
    public int Price() => Rate();
    //                    ^ d: picker src/Shop/Startup.cs:31, src/Shop/Pricing/Pricing.cs:19, src/Shop/Pricing/Pricing.cs:26; want src/Shop/Startup.cs:31 (#360)
}

public class Order
{
    public Courier Courier { get; set; } = new Courier("x");
    //     ^ d: picker src/Shop/Warehouse/Courier.cs:3; want src/Shop/Warehouse/Courier.cs:3 (#317)
}

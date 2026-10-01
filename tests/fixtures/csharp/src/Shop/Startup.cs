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
    //                    ^ d: src/Shop/Startup.cs:31
    //                    status: via Coupons
}

public class Order
{
    public Courier Courier { get; set; } = new Courier("x");
    //     ^ d: src/Shop/Warehouse/Courier.cs:3
    //             ^ d: !jump
    //             status: at a declaration
}

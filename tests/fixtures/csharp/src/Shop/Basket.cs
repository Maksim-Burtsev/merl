using Shop.Pricing;
//         ^ d: src/Shop/Pricing/Pricing.cs:1
using Shop.Warehouse;
using Rates = Shop.Pricing.Tariff;

namespace Shop;

public class Basket
{
    private readonly Tariff _tariff;
    //               ^ d: src/Shop/Pricing/Pricing.cs:12
    private readonly Coupon _coupon = new Coupon();
    //               ^ d: src/Shop/Pricing/Pricing.cs:24
    private int limit = 30;

    public Basket(Rates tariff) { _tariff = tariff; }
    //            ^ d: src/Shop/Basket.cs:4

    public int Gross() => Prices.Discount(_tariff.Rate());
    //                    ^ d: src/Shop/Pricing/Pricing.cs:42
    //                           ^ d: src/Shop/Pricing/Pricing.cs:46
    //                                    ^ d: src/Shop/Basket.cs:10
    //                                            ^ d: picker src/Shop/Pricing/Pricing.cs:19, src/Shop/Pricing/Pricing.cs:26, src/Shop/Startup.cs:31; want src/Shop/Pricing/Pricing.cs:19 (#352)

    public string Label() => _coupon.Describe();
    //                               ^ d: picker src/Shop/Pricing/Pricing.cs:20, src/Shop/Pricing/Pricing.cs:27; want src/Shop/Pricing/Pricing.cs:27 (#352)

    public bool Overweight(int grams)
    {
        var limit = Courier.Weigh(grams) + 20;
        //          ^ d: picker src/Shop/Startup.cs:38, src/Shop/Warehouse/Courier.cs:3; want src/Shop/Warehouse/Courier.cs:3 (#360)
        //                  ^ d: src/Shop/Warehouse/Courier.cs:9
        return grams > limit;
        //             ^ d: src/Shop/Basket.cs:30
    }

    public string Dispatch(Courier courier) => courier.Name;
    //                                         ^ d: src/Shop/Basket.cs:37
    //                                                 ^ d: src/Shop/Warehouse/Courier.cs:5

    public int Bonus(List<IPriced> items)
    {
        foreach (var item in items)
        {
            Prices.Changed += (s, e) => item.Price();
            //     ^ d: src/Shop/Pricing/Pricing.cs:44
            //                          ^ d: src/Shop/Basket.cs:43
        }
        return items.Sum(priced => priced.Price());
        //                         ^ d: src/Shop/Basket.cs:49
        //                                ^ d: picker src/Shop/Pricing/Pricing.cs:9, src/Shop/Pricing/Pricing.cs:21, src/Shop/Pricing/Pricing.cs:28, src/Shop/Startup.cs:32; want src/Shop/Pricing/Pricing.cs:9 (#352)
    }

    public Receipt Close(Stamp stamp, Voucher voucher, Money money, RateFn fn)
    //     ^ d: src/Shop/Pricing/Pricing.cs:33
    //                   ^ d: src/Shop/Pricing/Pricing.cs:34
    //                                ^ d: src/Shop/Pricing/Pricing.cs:35
    //                                                 ^ d: src/Shop/Pricing/Pricing.cs:36
    //                                                              ^ d: src/Shop/Pricing/Pricing.cs:40
    {
        var ledger = new Ledger<Tariff>();
        //               ^ d: src/Shop/Pricing/Pricing.cs:50
        return new Receipt(ledger.Count + money.Cents + fn(Offer.Cut));
        //                        ^ d: src/Shop/Pricing/Pricing.cs:53
        //                                      ^ d: src/Shop/Pricing/Pricing.cs:38
        //                                                 ^ d: src/Shop/Pricing/Pricing.cs:31
        //                                                       ^ d: src/Shop/Pricing/Pricing.cs:31
        // status: via Offer
    }

    public int Compare(Weights w, object o) => ((IComparable)w).CompareTo(o);
    //                 ^ d: src/Shop/Pricing/Pricing.cs:56
    //                                                          ^ d: src/Shop/Pricing/Pricing.cs:58

    public Courier Hire() => new Courier(name: "post");
    //                                   ^ d: src/Shop/Warehouse/Courier.cs:7

    public ScopeView Scope() => new ScopeView { Checked = true };
    //                                          ^ d: picker src/Shop/Warehouse/Courier.cs:25, src/Shop/Warehouse/Courier.cs:30; want src/Shop/Warehouse/Courier.cs:25 (#352)

    public Address Home() => new Address { Street = "Main" };
    //     ^ d: picker src/Shop/Warehouse/Courier.cs:33, src/Shop.Api/Address.cs:3; want src/Shop/Warehouse/Courier.cs:33 (#349)

    public int Coupons(Coupon coupon) => coupon.Price();
    //                                          ^ d: picker src/Shop/Pricing/Pricing.cs:9, src/Shop/Pricing/Pricing.cs:21, src/Shop/Pricing/Pricing.cs:28, src/Shop/Startup.cs:32; want src/Shop/Pricing/Pricing.cs:28 (#352)

    public Tier Level(bool gold) => gold ? Tier.Platinum : Tier.Diamond;
    //                                          ^ d: src/Shop/Pricing/Pricing.cs:65
    //                                                          ^ d: src/Shop/Pricing/Pricing.cs:66

    public Tier First() => Tier.Basic;
    //                          ^ d: src/Shop/Pricing/Pricing.cs:64
}

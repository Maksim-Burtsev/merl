namespace Shop.Api;

public class Address
{
    public string Street { get; set; } = "";

    public int Offered() => (int)Shop.Pricing.Offer.Cut;
    //                                              ^ d: src/Shop/Pricing/Pricing.cs:31
    // status: via Shop.Pricing.Offer
    // No `using Shop.Pricing;` here: this `Tier` is somebody else's.
    public object Level() => Tier.Gold;
    //                            ^ d: none
}

using Microsoft.AspNetCore.Authorization;
using Shop.Crest;

namespace Shop.Doors;

public class Wicket
{
    [Authorize(Roles = "admin")]
   //^ d: none
   //          ^ d: none
    public void Open() { }

    [Obsolete]
   //^ d: none
    public void Close() { }

    [ProjectAttr(Name = "home")]
   //^ d: src/Shop/Crest/Badges.cs:3
   //            ^ d: src/Shop/Crest/Badges.cs:7
    public void Home() { }

    [ProjectAttr("index")]
   //^ d: src/Shop/Crest/Badges.cs:3
    public void Index() { }

    [Local(Strict = true)]
   //^ d: src/Shop/Crest/Badges.cs:11
   //      ^ d: src/Shop/Crest/Badges.cs:13
    public void Latch() { }

    public object Make() => new Authorize();
    //                          ^ d: none

    public object Price() => new Tariff(1);
    //                           ^ d: src/Shop/Pricing/Pricing.cs:12
}

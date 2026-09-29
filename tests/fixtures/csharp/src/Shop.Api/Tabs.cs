using Shop.Api.Drawers;

namespace Shop.Api;

// #355: a member of a framework type, a private member or a local of somebody else's, all
// found by name alone, are no answer.
public class Tabs
{
    public Drawer Top { get; } = new Drawer();

    public string Badge(BindableObject view) => (string)view.GetValue(BadgeTextProperty);
    //                                                       ^ d: none

    public void Run(Drawer drawer)
    {
        JsonWebTokenHandler.DefaultInboundClaimTypeMap.Remove("sub");
        //                                             ^ d: none
        var status = HttpStatusCode.Created;
        //                          ^ d: none
        drawer.Lock();
        //     ^ d: none
        drawer.Empty();
        //     ^ d: src/Shop.Api/Drawers/Drawer.cs:7
        drawer.Oil();
        //     ^ d: src/Shop.Api/Drawers/Drawer.cs:8
        drawer.Dust();
        //     ^ d: src/Shop.Api/Drawers/Drawer.cs:9
        drawer.Wax();
        //     ^ d: src/Shop.Api/Drawers/Drawer.cs:10
        Top.Empty();
        //  ^ d: src/Shop.Api/Drawers/Drawer.cs:7
    }

    public void Fill(Options options)
    {
        options.Enabled = true;
        // ^ d: none
    }

    public void Show() => Tidy();
    //                    ^ d: src/Shop.Api/Tabs.cs:43

    void Tidy() { }
}

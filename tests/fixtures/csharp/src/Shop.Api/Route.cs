namespace Shop.Api;

// #466: a file of the enum's own namespace sees it with no `using`, as does one of a namespace
// inside it (`Handlers/Cart.cs`); every file sees an enum of the global namespace (`Shop/Mood.cs`).
public enum Route
{
    Home,
    Cart,
}

public static class Routes
{
    public static object Start() => Route.Home;
    //                                    ^ d: src/Shop.Api/Route.cs:7

    public static object Feel() => Mood.Loud;
    //                                  ^ d: src/Shop/Mood.cs:5
}

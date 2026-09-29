namespace Shop;

public static class Labels
{
    public static string Loud() => string.Join(separator: ",", values: new[] { "a" });
    //                                         ^ d: none
    //                                         status: separator: argument label
}

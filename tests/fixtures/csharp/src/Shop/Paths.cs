namespace Shop;

public static class Paths
{
    // A verbatim string ending in a backslash ends at its second quote (#475).
    const string Root = @"C:\";

    public static string Data(string name) => JoinUnder(Root, name);
    //                                        ^ d: src/Shop/Paths.cs:11

    static string JoinUnder(string a, string b) => a + b;
}

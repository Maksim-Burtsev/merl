namespace Shop.Warehouse;

public class Courier
{
    public string Name { get; }

    public Courier(string name) { Name = name; }

    public static int Weigh(int grams) => grams / 1000;
}

public static class Notes
{
    public const string Banner = @"
public class Courier {
";

    public const string Raw = """
        public static int Weigh(int grams) => 0;
        """;
}

public class ScopeView
{
    public bool Checked { get; set; }
}

public class Toggle
{
    public bool Checked { get; set; }
}

public class Address
{
    public string Street { get; set; } = "";
}

class MockSettings
{
    void Remove(string key) { }
}

public class Animation
{
    public int Delay { get; set; }
}

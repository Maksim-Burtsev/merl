namespace App;

public class Basket
{
    private readonly List<string> items = new();

    public void Add(string item)
    {
        items.Add(item);
    }

    public bool Contains(string item) => items.Contains(item);
}

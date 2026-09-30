namespace Shop.Api.Drawers;

// #355: a partial type whose private members its other part reaches, and nothing else does.
public partial class Drawer
{
    void Lock() { }
    public void Empty() { }
    protected void Oil() { }
    internal void Dust() { }
    private protected void Wax() { }
}

class ChatState
{
    void Load()
    {
        static string GetValue(string key) => key;
        var options = new Options();
    }

    public bool Created { get; set; }
}

public class Options
{
    public bool Enabled { get; set; }
}

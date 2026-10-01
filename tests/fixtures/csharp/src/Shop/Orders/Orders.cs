namespace Shop.Orders;

public class Buyer
{
    public Buyer(string id) { }
}

public class Purchase
{
    public Buyer Buyer { get; set; }
    //     ^ d: src/Shop/Orders/Orders.cs:3

    public List<Buyer> All(object o) => new() { (Buyer)o };
    //          ^ d: src/Shop/Orders/Orders.cs:3
    //                                           ^ d: src/Shop/Orders/Orders.cs:3

    public bool Known(object o) => o is Buyer;
    //                                  ^ d: src/Shop/Orders/Orders.cs:3
}

public interface IBuyerRepository
{
    Buyer Update(Buyer buyer);
    //           ^ d: src/Shop/Orders/Orders.cs:3
}

public class ViewModelBase
{
    protected async Task IsBusyFor(Func<Task> work)
    {
        await work();
    }
}

public class OrderViewModel : ViewModelBase
//                            ^ d: src/Shop/Orders/Orders.cs:27
{
    public async Task Load()
    {
        await IsBusyFor(LoadAsync);
        //    ^ d: src/Shop/Orders/Orders.cs:29
        //    status: via OrderViewModel
        //              ^ d: src/Shop/Orders/Orders.cs:46
    }

    private Task LoadAsync() => Task.CompletedTask;
}

public class Spinner
{
    public Task IsBusyFor(Func<Task> work) => work();
}

public sealed class GetBasketRequest
{
    private object _unknownFields = new();

    public override bool Equals(object other)
    {
        return Equals(other as GetBasketRequest);
        //     ^ d: picker src/Shop/Orders/Orders.cs:58, src/Shop/Orders/Orders.cs:64
    }

    public bool Equals(GetBasketRequest other)
    {
        return Equals(_unknownFields, other._unknownFields);
        //     ^ d: none
    }
}

public class BasketViewModel : ViewModelBase
{
    private readonly object _service;

    public BasketViewModel(
    //     ^ d: !jump
    //     status: at a declaration, 1 other by name
        object service,
        object navigation)
    {
        _service = service;
    }
}

public static class Screens
{
    public static object Open() => new BasketViewModel(null, null);
    //                                 ^ d: src/Shop/Orders/Orders.cs:71
}

public class Outer
{
    private static int Limit() => 3;

    public class Inner
    {
        public int Read() => Limit();
        //                   ^ d: src/Shop/Orders/Orders.cs:93
        //                   status: via Outer
    }
}

public static class Ledgers
{
    public static object All() => new List<LedgerService>();
    //                                     ^ d: src/Shop/Orders/Ledgers.cs:4
}

public class Tally
{
    public Tally() { }
    //     ^ d: picker src/Shop/Orders/Orders.cs:109, src/Shop/Orders/Orders.cs:115
    //     status: at a declaration, 2 others by name

    public Tally(int start) { }
}

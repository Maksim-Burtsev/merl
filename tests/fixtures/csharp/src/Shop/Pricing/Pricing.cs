namespace Shop.Pricing;

/* A block comment that reads like code declares nothing:
public class Basket {
*/

public interface IPriced
{
    int Price();
}

public class Tariff : IPriced
{
    public const int RateCap = 100;
    public int Base { get; }

    public Tariff(int start) { Base = start; }

    public int Rate() => 1;
    public string Describe() { return "tariff"; }
    public int Price() => Rate();
}

public sealed class Coupon : IPriced
{
    public int Rate() => 2;
    public string Describe() => "coupon";
    public int Price() => Rate();
}

public enum Offer { Plain, Cut }

public record Receipt(int Total);
public record struct Stamp(int Code);
public record class Voucher(string Code);
public readonly struct Money
{
    public readonly int Cents;
}
public delegate int RateFn(int total);

public static class Prices
{
    public static event EventHandler? Changed;

    public static int Discount(int total) => Math.Min(total, Tariff.RateCap) - 1;
}

[Serializable]
public sealed partial class Ledger<T> where T : IPriced
{
    private readonly List<T> _items = new();
    public int Count => _items.Count;
}

public class Weights : IComparable
{
    int IComparable.CompareTo(object? o) => 0;
}

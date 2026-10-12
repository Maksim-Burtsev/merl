// Ledger.cs: the accounts of a small shop
#nullable enable
#region Usings
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using static System.Math;
using Map = System.Collections.Generic.Dictionary<string, long>;
#endregion

namespace Shop.Books;

public enum Kind : byte { Debit, Credit }

public record Entry(string Memo, long Cents, Kind Kind = Kind.Debit);

public readonly record struct Point(int X, int Y);

[Serializable]
[Obsolete("use Books", error: false)]
public sealed partial class Ledger<T> : Base, IBook, IDisposable where T : struct, IComparable<T>
{
    public const int Limit = 1_000;
    private static readonly string Header = "memo,cents\n";
    private readonly List<Entry> _entries = new();
    private volatile int _reads;
    protected char Sep = ',';

    public event EventHandler<Entry>? Posted;

    public string Name { get; init; } = "main";
    public long this[int i] => _entries[i].Cents;

    /// <summary>A ledger with the given name.</summary>
    public Ledger(string name) : base(name)
    {
        Name = name ?? throw new ArgumentNullException(nameof(name));
    }

    public override long Total()
    {
        long sum = 0L;
        foreach (var e in _entries)
        {
            sum += e.Kind == Kind.Credit ? -e.Cents : e.Cents;
        }
        return checked(sum);
    }

    public Entry? Find(string memo) =>
        _entries.Where(e => e.Memo == memo).Select(e => e with { Cents = Abs(e.Cents) }).FirstOrDefault();

    public async Task<int> PostAsync(Entry e, CancellationToken token = default)
    {
        try
        {
            if (e.Cents < 0) throw new ArgumentException("negative");
            lock (_entries) { _entries.Add(e); }
            Posted?.Invoke(this, e);
            await Task.Delay(10, token).ConfigureAwait(false);
        }
        catch (ArgumentException err) when (err.Message is not null)
        {
            Console.Error.WriteLine($"post: {err.Message,-10} at {DateTime.Now:HH:mm}");
        }
        finally
        {
            _reads++;
        }
        return _entries.Count;
    }

    static string Describe(object? o) => o switch
    {
        int i when i > 0 => "positive " + i,
        string { Length: > 3 } s => string.Format("text {0}", s),
        null => "nothing",
        _ => "other",
    };

    internal static unsafe int First<TItem>(IList<TItem> list, ref int seen, out bool found, params int[] rest)
    {
        var text = @"memo,cents
cash,100";
        var raw = """
            memo,cents
            """;
        double d = 0x1F + 1e-3 + 0b1010 + 3.5f + 2m;
        int* p = stackalloc int[4];
        found = list is { Count: > 0 } && text.Length > sizeof(int) && raw is not null;
        goto done;
    done:
        return default;
    }

    public void Dispose() => GC.SuppressFinalize(this);
}

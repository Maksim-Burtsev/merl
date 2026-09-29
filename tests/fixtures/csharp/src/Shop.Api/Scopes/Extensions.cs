namespace Shop.Api.Scopes;

// #345: a parameter, a lambda's parameter, a loop variable or a local resolves in its own method.
public static class Extensions
{
    public static void AddAuth(IServiceCollection services, string callBackUrl)
    {
        services.AddOpenIdConnect(options =>
        {
            options.SignedOutRedirectUri = callBackUrl;
            // ^ d: src/Shop.Api/Scopes/Extensions.cs:8
            //                             ^ d: src/Shop.Api/Scopes/Extensions.cs:6
        });
        foreach (var item in services)
        {
            Console.WriteLine(item.ServiceType);
            //                ^ d: src/Shop.Api/Scopes/Extensions.cs:14
        }
    }

    public static int Tally(
        this Ledger ledger,
        int start = 0)
    {
        var sum = start;
        //        ^ d: src/Shop.Api/Scopes/Extensions.cs:23
        for (var i = 0; i < ledger.Count; i++)
        {
            sum += i;
            //     ^ d: src/Shop.Api/Scopes/Extensions.cs:27
            //^ d: src/Shop.Api/Scopes/Extensions.cs:25
        }
        try
        {
            using var stream = Open(ledger);
            //                      ^ d: src/Shop.Api/Scopes/Extensions.cs:22
            stream.Flush();
            // ^ d: src/Shop.Api/Scopes/Extensions.cs:35
        }
        catch (IOException error)
        {
            Log(error);
            //  ^ d: src/Shop.Api/Scopes/Extensions.cs:40
        }
        using (var reader = Read(ledger))
        {
            reader.Close();
            // ^ d: src/Shop.Api/Scopes/Extensions.cs:45
        }
        if (int.TryParse("1", out var parsed))
        {
            sum += parsed;
            //     ^ d: src/Shop.Api/Scopes/Extensions.cs:50
        }
        Take(out Receipt given);
        sum += given.Total;
        //     ^ d: src/Shop.Api/Scopes/Extensions.cs:55
        object boxed = ledger;
        if (boxed is Stamp seal)
        {
            sum += seal.Code;
            //     ^ d: src/Shop.Api/Scopes/Extensions.cs:59
        }
        switch (boxed)
        {
            case Voucher ticket:
                sum += ticket.Code.Length;
                //     ^ d: src/Shop.Api/Scopes/Extensions.cs:66
                break;
        }
        int amount;
        amount = sum;
        // ^ d: src/Shop.Api/Scopes/Extensions.cs:71
        var (left, right) = (1, 2);
        return left + right + amount;
        //     ^ d: none
    }

    public static Func<int, int, int> Adders() => (a, b) => a + b;
    //                                                      ^ d: src/Shop.Api/Scopes/Extensions.cs:79
    public static Func<int, int> Typed() => (int x) => x;
    //                                                 ^ d: src/Shop.Api/Scopes/Extensions.cs:81
    public static Func<int, Task<int>> Later() => async n => await Task.FromResult(n);
    //                                                                             ^ d: src/Shop.Api/Scopes/Extensions.cs:83

    static int Heavy = 1;

    // A lambda's parameter on the cursor's line binds inside that lambda only.
    public static int Weigh(int[] xs) => xs.Count(Heavy => Heavy > 1) + Heavy;
    //                                                     ^ d: src/Shop.Api/Scopes/Extensions.cs:89
    //                                                                  ^ d: src/Shop.Api/Scopes/Extensions.cs:86
}

public class OrderServices(IMediator mediator, ILogger logger)
{
    public IMediator Mediator { get; } = mediator;
    //                                   ^ d: src/Shop.Api/Scopes/Extensions.cs:94

    public void Note() => logger.LogInformation("note");
    //                    ^ d: src/Shop.Api/Scopes/Extensions.cs:94
}

public class ServiceTests
{
    public void A()
    {
        var service = new OrderServices(null, null);
        service.Note();
    }

    public void B()
    {
        var service = new OrderServices(null, null);
        service.Note();
        // ^ d: src/Shop.Api/Scopes/Extensions.cs:113
    }
}

// Two lambdas on one line bind a parameter each; a parameter's own declaration is its answer.
public static class Chains
{
    public static int Both(int[] xs) => xs.Where(v => v > 0).Select(v => v * 2).Sum();
    //                                                              ^ d: src/Shop.Api/Scopes/Extensions.cs:122
    //                                                                   ^ d: src/Shop.Api/Scopes/Extensions.cs:122

    public static int Twice(int count) => count * 2;
    //                          ^ d: src/Shop.Api/Scopes/Extensions.cs:126
}

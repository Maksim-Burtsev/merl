namespace App;

public static class Totals
{
    public static List<int> Doubled(List<int> prices)
    {
        var result = new List<int>();
        List<int> seen = new();
        foreach (var p in prices)
        {
            result.Add(p * 2);
            //     ^ d: none
            seen.Add(p);
            //   ^ d: none
        }
        var text = "a,b";
        return text.Contains("a") ? result : seen;
        //          ^ d: none
    }
}

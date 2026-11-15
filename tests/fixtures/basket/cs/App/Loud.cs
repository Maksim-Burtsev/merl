namespace App;

public static class Loud
{
    public static string Run()
    {
        var s = "a";
        return s.Shout();
        //       ^ d: cs/App/Ext.cs:5
    }
}

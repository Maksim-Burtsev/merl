// Every construct `f` folds in C#, and the cases that once broke it.
using System;  // f: 2-3
using System.Collections.Generic;  // f: 2-3
// a comment ends a run of usings
using System.Linq;  // f: 5-7

using System.Text;  // f: 5-7

namespace Shop.Orders  // f: 10-146
{  // f: 10-146
    public interface IRepository<T> where T : class  // f: 12-14
    {  // f: 12-14
        T Find(int id);  // f: 12-14
    }  // f: 12-14

    public record Line(string Sku, int Count)  // f: 17-19
    {  // f: 17-19
        public int Total => Count;  // f: 17-19
    }  // f: 17-19

    public enum Status  // f: 22-25
    {  // f: 22-25
        New,  // f: 22-25
        Paid,  // f: 22-25
    }  // f: 22-25

    public class Order : Base, IDisposable  // f: 28-145
    {  // f: 28-145
        private readonly List<Line> _lines = new();  // f: 28-145

        public int Count  // f: 32-37
        {  // f: 32-37
            get  // f: 34-36
            {  // f: 34-36
                return _lines.Count;  // f: 34-36
            }  // f: 34-36
        }  // f: 32-37

        public new async Task CloseAsync()  // f: 40-42
        {  // f: 40-42
            await Task.Yield();  // f: 40-42
        }  // f: 40-42

        public static T Make<T>(object seed)  // f: 46-52
            where T : Base, new()  // f: 46-52
        {  // f: 46-52
            if (seed is not string text)  // f: 48-50
            {  // f: 48-50
                return new T();  // f: 46-52
            }  // f: 46-52
            return new T();  // f: 46-52
        }  // f: 46-52

        public string Describe(object shape) => shape switch  // f: 28-145
        {  // f: 28-145
            { Length: 0 } => "empty",  // f: 28-145
            Line { Count: > 1 } => "many",  // f: 28-145
            _ => "other",  // f: 28-145
        };  // f: 28-145

        public void Fill(int n)  // f: 62-142
        {  // f: 62-142
#if DEBUG  // f: 63-70
            Console.WriteLine("debug");  // f: 62-142
#elif TRACE  // f: 65-69
            Console.WriteLine("trace");  // f: 62-142

#else  // f: 68-69
            Console.WriteLine("release");  // f: 62-142
#endif  // f: 62-142
            var json = $@"{{  // f: 62-142
  ""count"": {n}  // f: 62-142
}}";  // f: 62-142
            var raw = """  // f: 62-142
                { not a block  // f: 62-142
                """;  // f: 62-142
            var brace = '{';  // f: 62-142
            // }  // f: 62-142
            var order = new Order  // f: 80-86
            {  // f: 80-86
                Name = "a",  // f: 62-142
                Tags =  // f: 62-142
                {  // f: 62-142
                    "x",  // f: 62-142
                },  // f: 62-142
            };  // f: 62-142
            var same = new()  // f: 62-142
            {  // f: 62-142
                Name = "b",  // f: 62-142
            };  // f: 62-142
            var numbers = new int[]  // f: 62-142
            {  // f: 62-142
                1, 2,  // f: 62-142
            };  // f: 62-142
            var anonymous = new  // f: 62-142
            {  // f: 62-142
                A = 1,  // f: 62-142
            };  // f: 62-142
            var copy = order with  // f: 62-142
            {  // f: 62-142
                Name = "c",  // f: 62-142
            };  // f: 62-142
            var map = new Dictionary<int, string>  // f: 104-108
            {  // f: 104-108
                {  // f: 62-142
                    1, "one"  // f: 62-142
                },  // f: 62-142
            };  // f: 62-142
            Action log = () =>  // f: 110-112
            {  // f: 110-112
                Console.WriteLine(json);  // f: 110-112
            };  // f: 110-112
            foreach (var line in _lines)  // f: 114-124
            {  // f: 114-124
                switch (line.Count)  // f: 116-123
                {  // f: 116-123
                    case 0:  // f: 118-120
                    {  // f: 118-120
                        break;  // f: 62-142
                    }  // f: 62-142
                    default:  // f: 62-142
                        break;  // f: 62-142
                }  // f: 62-142
            }  // f: 62-142
            try  // f: 126-128
            {  // f: 126-128
                log();  // f: 62-142
            }  // f: 62-142
            catch (Exception)  // f: 130-132
            {  // f: 130-132
                throw;  // f: 62-142
            }  // f: 62-142
            finally  // f: 134-136
            {  // f: 134-136
                Local();  // f: 62-142
            }  // f: 62-142

            void Local()  // f: 139-141
            {  // f: 139-141
                Console.WriteLine(raw + brace);  // f: 139-141
            }  // f: 139-141
        }  // f: 62-142

        public void Dispose() { }  // f: 28-145
    }  // f: 28-145
}  // f: 10-146

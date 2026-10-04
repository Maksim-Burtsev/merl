// Every construct `f` folds in C#, and the cases that once broke it.
using System;  // f: 2-3
using System.Collections.Generic;  // f: 2-3
// a comment ends a run of usings
using System.Linq;  // f: 5-7

using System.Text;  // f: 5-7

namespace Shop.Orders  // f: 10-148
{  // f: 10-148
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

    public class Order : Base, IDisposable  // f: 28-147
    {  // f: 28-147
        private readonly List<Line> _lines = new();  // f: 28-147

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

        public string Describe(object shape) => shape switch  // f: 28-147
        {  // f: 28-147
            { Length: 0 } => "empty",  // f: 28-147
            Line { Count: > 1 } => "many",  // f: 28-147
            _ => "other",  // f: 28-147
        };  // f: 28-147

        public void Fill(int n)  // f: 62-144
        {  // f: 62-144
#if DEBUG  // f: 63-70
            Console.WriteLine("debug");  // f: 62-144
#elif TRACE  // f: 65-69
            Console.WriteLine("trace");  // f: 62-144

#else  // f: 68-69
            Console.WriteLine("release");  // f: 62-144
#endif  // f: 62-144
            var json = $@"{{  // f: 62-144
  ""count"": {n}  // f: 62-144
}}";  // f: 62-144
            var raw = """  // f: 62-144
                { not a block  // f: 62-144
                """;  // f: 62-144
            var brace = '{';  // f: 62-144
            var lookup = new Dictionary<string, string>();  // f: 62-144
            var hole = $"{lookup["}"]} {{";  // f: 62-144
            // }  // f: 62-144
            var order = new Order  // f: 82-88
            {  // f: 82-88
                Name = "a",  // f: 62-144
                Tags =  // f: 62-144
                {  // f: 62-144
                    "x",  // f: 62-144
                },  // f: 62-144
            };  // f: 62-144
            var same = new()  // f: 62-144
            {  // f: 62-144
                Name = "b",  // f: 62-144
            };  // f: 62-144
            var numbers = new int[]  // f: 62-144
            {  // f: 62-144
                1, 2,  // f: 62-144
            };  // f: 62-144
            var anonymous = new  // f: 62-144
            {  // f: 62-144
                A = 1,  // f: 62-144
            };  // f: 62-144
            var copy = order with  // f: 62-144
            {  // f: 62-144
                Name = "c",  // f: 62-144
            };  // f: 62-144
            var map = new Dictionary<int, string>  // f: 106-110
            {  // f: 106-110
                {  // f: 62-144
                    1, "one"  // f: 62-144
                },  // f: 62-144
            };  // f: 62-144
            Action log = () =>  // f: 62-144
            {  // f: 112-114
                Console.WriteLine(json);  // f: 112-114
            };  // f: 112-114
            foreach (var line in _lines)  // f: 116-126
            {  // f: 116-126
                switch (line.Count)  // f: 118-125
                {  // f: 118-125
                    case 0:  // f: 62-144
                    {  // f: 120-122
                        break;  // f: 62-144
                    }  // f: 62-144
                    default:  // f: 62-144
                        break;  // f: 62-144
                }  // f: 62-144
            }  // f: 62-144
            try  // f: 128-130
            {  // f: 128-130
                log();  // f: 62-144
            }  // f: 62-144
            catch (Exception)  // f: 132-134
            {  // f: 132-134
                throw;  // f: 62-144
            }  // f: 62-144
            finally  // f: 136-138
            {  // f: 136-138
                Local();  // f: 62-144
            }  // f: 62-144

            void Local()  // f: 141-143
            {  // f: 141-143
                Console.WriteLine(raw + brace);  // f: 141-143
            }  // f: 141-143
        }  // f: 62-144

        public void Dispose() { }  // f: 28-147
    }  // f: 28-147
}  // f: 10-148

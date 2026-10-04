namespace Shop.Orders

module Ledger =
    [<Literal>]
    let Limit = 10
    let private fee = 2
    let inline twice x = x * 2
    let mutable counter = 0
    let add acc x = acc + x
    let rec loop acc xs =
        match xs with
        | [] -> acc
        | x :: rest -> helper acc x rest
    and helper acc x rest = loop (acc + x) rest
//                          ^ d: src/Pricing.fs:10

type Shape =
    | Circle of float
    | Square of float

type Status = Active | Closed

type Account(id: int) =
    let mutable count = 0
    val mutable seen : int
    member this.Total = count
    member _.Id = id
    override this.ToString() = "account"
    static member Create(id) = Account(id)

[<AbstractClass>]
type Figure() =
    abstract member Area : float
    default this.Area = 0.0

type IRepo =
    abstract Find : int -> string

type Coupon = { Code: string; mutable Rate: int }

exception NotFound of string

module Tariff =
    let ``returns "the" rate`` () = 1
    let product xs = List.reduce (*) xs
    let tariffRate = 3
    (* A nested comment (* inside *) holds:
    let fakeRate = 0
    *)
    let verbatim = @"C:\path\""
    let fakeVerbatim = 0
    "
    let triple = """
    let fakeTriple = 0
    """
    let interpolated = $"{tariffRate}"

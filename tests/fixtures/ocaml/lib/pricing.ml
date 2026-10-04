exception Invalid_price of int

external length : string -> int = "caml_string_length"

type shape =
  | Circle of float
  | Square of float

type _ expr =
  | Lit : int -> int expr
  | Add : int expr * int expr -> int expr

type status = Active | Closed

type account = {
  owner : string;
  mutable total : int;
}
and entry = { amount : int }

module type S = sig
  val weigh : int -> int
end

module Ledger = struct
  let add acc x = acc + x
  let rec loop acc = function
    | [] -> acc
    | x :: rest -> helper acc x rest
  and helper acc x rest = loop (acc + x) rest
(*                        ^ d: lib/pricing.ml:27 *)
(*                        status: (in this file) *)
end

module Sealed : sig
  val tally : int list -> int
end = struct
  let tally xs = List.fold_left ( + ) 0 xs
end

module Make (X : S) = struct
  let weigh_twice n = X.weigh (X.weigh n)
end

module Courier = Make (struct let weigh n = n * 2 end)
(*               ^ d: lib/pricing.ml:41 *)

class point x = object
  val mutable px = x
  method get_x = px
(*               ^ d: lib/pricing.ml:49 *)
end

let tariff_of = function `Active -> 1 | `Closed -> 0

let coupon_rate = 3

(* A nested comment (* inside *) holds:
let fake_rate = 0
*)

let sql = {sql|
  let fake_query = 1
|sql}

let quoted = "(* not a comment"

let discount = coupon_rate

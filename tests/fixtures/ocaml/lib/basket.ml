open Pricing.Ledger
(*           ^ d: lib/pricing.ml:25 *)
(*           status: via Pricing *)

let area (s : shape) =
(*            ^ d: lib/pricing.ml:5 *)
  match s with
  | Circle r -> r *. r
(*  ^ d: lib/pricing.ml:6 *)
  | Square a -> a *. a
(*  ^ d: lib/pricing.ml:7 *)

let check n = if n < 0 then raise (Invalid_price n) else length "x"
(*                                 ^ d: lib/pricing.ml:1 *)
(*                                                        ^ d: lib/pricing.ml:3 *)

let lit = Lit 1
(*        ^ d: lib/pricing.ml:10 *)

let closed = Closed
(*           ^ d: lib/pricing.ml:13 *)

let owner_of (a : account) = a.owner
(*                             ^ d: lib/pricing.ml:16 *)

let reset (a : account) = { a with total = 0 }
(*                                 ^ d: lib/pricing.ml:17 *)

let first (e : entry) = e.amount
(*             ^ d: lib/pricing.ml:19 *)
(*                        ^ d: lib/pricing.ml:19 *)

module Weigh (X : S) = struct end
(*                ^ d: lib/pricing.ml:21 *)

let sum = Ledger.add 1 2
(*        ^ d: lib/pricing.ml:25 *)
(*               ^ d: lib/pricing.ml:26 *)
(*               status: add (via Ledger) *)

let qualified = Pricing.Ledger.loop 0 [ 1 ]
(*                             ^ d: lib/pricing.ml:27 *)

let sealed = Sealed.tally [ 1; 2 ]
(*                  ^ d: lib/pricing.ml:38 *)

let twice = Courier.weigh_twice 2
(*          ^ d: lib/pricing.ml:45 *)

let p = new point 3
(*          ^ d: lib/pricing.ml:48 *)

let x = p#get_x
(*         ^ d: lib/pricing.ml:50 *)

let total xs =
  let acc = ref 0 in
  List.iter (fun x -> acc := !acc + x) xs;
  !acc
(* ^ d: lib/basket.ml:57 *)
(* status: (local) *)

let rate = Pricing.tariff_of `Active + coupon_rate
(*                                     ^ d: lib/pricing.ml:56 *)

let faked = fake_rate + fake_query
(*          ^ d: none *)
(*                      ^ d: none *)

let label m =
  let prefix = "Total: " in
  prefix ^ Money.format_price m
(*^ d: lib/cart.ml:2 *)
(*status: prefix (local) *)
(*               ^ d: lib/money.ml:3 *)
(*               status: format_price: via Money *)

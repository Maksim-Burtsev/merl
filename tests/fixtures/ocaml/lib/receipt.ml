open Money

let line m = format_price m
(*           ^ d: lib/money.ml:3 *)

let format_price m = "local " ^ Money.format_price m

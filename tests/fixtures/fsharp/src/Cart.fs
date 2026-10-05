module Shop.Cart

open Shop.Money

let label m =
    let prefix = "Total: "
    prefix + formatPrice m
//  ^ d: src/Cart.fs:6
//  status: prefix (local)
//           ^ d: src/Money.fs:5
//           status: formatPrice: by name, 1 match

let total xs =
    let acc = List.sum xs
    acc
//  ^ d: src/Cart.fs:14

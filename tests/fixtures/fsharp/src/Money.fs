module Shop.Money

type Money = { Cents: int }

let formatPrice (m: Money) = sprintf "$%.2f" (float m.Cents / 100.)

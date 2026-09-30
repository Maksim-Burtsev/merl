package cart

import (
	wh "example.com/fixture/shop"
)

func Checkout() int {
	b := wh.NewBasket(wh.Tariff{})
	//      ^ d: shop/basket.go:11
	//                   ^ d: shop/pricing.go:23
	c := wh.Courier{Name: "post"}
	//              ^ d: shop/warehouse.go:9
	return b.Bonus() + wh.Weigh(len(c.Name))
	//       ^ d: shop/basket.go:25
	//                 ^ d: cart/cart.go:4
	//                    ^ d: shop/warehouse.go:12
	//                                ^ d: shop/warehouse.go:9
}

func Currency() string {
	return wh.Euro
	//        ^ d: shop/pricing.go:11
}

func Dollar() string { return "USD" }

func Count() int {
	count := 1
	return count
}

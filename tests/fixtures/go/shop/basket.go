package shop

var weightLimit = 30

type Basket struct {
	Tariff Tariff
	coupon *Coupon
	Owner  string
}

func NewBasket(t Tariff) *Basket {
	return &Basket{Tariff: t, coupon: &Coupon{}}
	//       ^ d: shop/basket.go:5
	//             ^ d: shop/pricing.go:23; want shop/basket.go:6 (#327)
}

func (b *Basket) Gross() Money {
//                       ^ d: none; want shop/pricing.go:20 (#326)
	return Money(Discount(b.Tariff.Rate()))
	//           ^ d: shop/pricing.go:35
	//                      ^ d: shop/basket.go:6
	//                             ^ d: picker shop/pricing.go:27, shop/pricing.go:32; want shop/pricing.go:27 (#453)
}

func (b *Basket) Bonus() int {
	return b.coupon.Rate() + int(b.Gross())
	//              ^ d: picker shop/pricing.go:27, shop/pricing.go:32; want shop/pricing.go:32 (#453)
	//                             ^ d: shop/basket.go:17
}

func DescribeAny(p interface{ Describe() string }) string {
	return p.Describe() + Describe("x")
	//       ^ d: picker shop/pricing.go:28, shop/pricing.go:33
	//                    ^ d: picker shop/pricing.go:28, shop/pricing.go:33, shop/warehouse.go:15; want shop/warehouse.go:15 (#332)
}

func Restock(Discount int) int {
	return Discount + weightLimit
	//     ^ d: shop/basket.go:37
	//                ^ d: shop/basket.go:3
}

func Overweight(grams int) bool {
	weightLimit := 50
	return Weigh(grams) > weightLimit
	//     ^ d: shop/warehouse.go:12
	//                    ^ d: shop/basket.go:44
}

func Currency() string {
	return Euro + Dollar
	//     ^ d: none; want shop/pricing.go:11 (#326)
	//            ^ d: cart/cart.go:25; want shop/pricing.go:12 (#326)
}

func Fallback() Tariff {
	return DefaultTariff
	//     ^ d: none; want shop/pricing.go:16 (#326)
}

func PriceOf(p Priced) int {
	return p.Price() + RateCap
	//       ^ d: shop/pricing.go:38
	//                 ^ d: shop/pricing.go:8
}

func Labelled(input string) int {
	count := 0
scan:
	for i := 0; i < len(input); i++ {
		if input[i] == 'x' {
			continue scan
		}
		count++
		//^ d: picker shop/basket.go:68, cart/cart.go:28, results.go:5; want shop/basket.go:68 (#330)
	}
	return count
}

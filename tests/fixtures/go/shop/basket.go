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
	//             ^ d: shop/basket.go:6
}

func (b *Basket) Gross() Money {
//                       ^ d: shop/pricing.go:20
	return Money(Discount(b.Tariff.Rate()))
	//           ^ d: shop/pricing.go:35
	//                      ^ d: shop/basket.go:6
	//                             ^ d: shop/pricing.go:27
}

func (b *Basket) Bonus() int {
	return b.coupon.Rate() + int(b.Gross())
	//              ^ d: shop/pricing.go:32
	//                             ^ d: shop/basket.go:17
}

func DescribeAny(p interface{ Describe() string }) string {
	return p.Describe() + Describe("x")
	//       ^ d: picker shop/pricing.go:28, shop/pricing.go:33
	//                    ^ d: shop/warehouse.go:15
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
	//     ^ d: shop/pricing.go:11
	//            ^ d: shop/pricing.go:12
}

func Fallback() Tariff {
	return DefaultTariff
	//     ^ d: shop/pricing.go:16
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
		//^ d: shop/basket.go:68
	}
	return count
}

func Quota(quota int) int {
outer:
	for i := 0; i < quota; i++ {
		continue outer
	}
	return quota
	//     ^ d: shop/basket.go:80
}

const Street = "s"

func Cut(xs []int, Street int) []int { return xs[Street:] }
//                                               ^ d: shop/basket.go:91

func Uncut(xs []int) []int { return xs[Street:] }
//                                     ^ d: shop/basket.go:89

const Lane = "l"

// A `func(...)` type in the signature names nothing in the body.
func Each(fn func(Lane string)) { fn(Lane) }
//                                   ^ d: shop/basket.go:97

func Maker() func(Lane string) { return Each(Lane) }
//                                           ^ d: shop/basket.go:97

func Walk(Lane string, visit func(Lane string, depth int)) { visit(Lane, 0) }
//                                                                 ^ d: shop/basket.go:106

// A parameter's type named like the method it is a parameter of is the type (#536).
type Parcel string

type Porter interface {
	Parcel(p Parcel) error
	//       ^ d: shop/basket.go:110
}

type Crate struct{}

func (c Crate) Parcel(p *Parcel) error { return nil }
//                       ^ d: shop/basket.go:110

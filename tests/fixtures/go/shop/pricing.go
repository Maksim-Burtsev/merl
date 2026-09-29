// Package shop is the shop of #307: every `d` case carries its answer in a comment under it.
package shop

/*
func Discount(total int) int {
*/

const RateCap = 100

const (
	Euro   = "EUR"
	Dollar = "USD"
)

var (
	DefaultTariff = Tariff{}
)

type (
	Money int
)

type Tariff struct {
	Base int
}

func (t Tariff) Rate() int        { return 1 }
func (t Tariff) Describe() string { return "tariff" }

type Coupon struct{ Code string }

func (c *Coupon) Rate() int        { return 2 }
func (c *Coupon) Describe() string { return "coupon" }

func Discount(total int) int { return min(total-1, RateCap) }

type Priced interface {
	Price() int
}

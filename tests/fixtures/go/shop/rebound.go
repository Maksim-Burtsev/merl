package shop

func ReboundIf(t Tariff) {
	if t := (&Coupon{}); t != nil { t.Rate() }
	//                                ^ d: picker shop/pricing.go:27, shop/pricing.go:32
}

func ReboundBlock(t Tariff) {
	{ t := &Coupon{}; t.Rate() }
	//                  ^ d: picker shop/pricing.go:27, shop/pricing.go:32
	{ var t *Coupon; t.Rate() }
	//                 ^ d: picker shop/pricing.go:27, shop/pricing.go:32
}

func ReboundRange(t Tariff, cs []*Coupon, ch chan *Coupon) {
	for _, t := range cs { t.Rate() }
	//                       ^ d: picker shop/pricing.go:27, shop/pricing.go:32
	select { case t := <-ch: t.Rate() }
	//                         ^ d: picker shop/pricing.go:27, shop/pricing.go:32
}

func ReboundTypeSwitch(t Tariff, a any) {
	switch t := a.(type) { case *Coupon: t.Rate() }
	//                                     ^ d: picker shop/pricing.go:27, shop/pricing.go:32
}

func ReboundOuter(t Tariff) {
	if t := t.Rate(); t > 0 { _ = t }
	//          ^ d: shop/pricing.go:27
}

func ReboundSwitch(t Tariff) {
	switch t := (&Coupon{}); {
	default:
		t.Rate()
		//^ d: shop/pricing.go:32
	}
}

type rebate struct{ t int }

func ReboundControls(t Tariff, cs []*Coupon) {
	if true { t := &Coupon{}; t.Rate() }; t.Rate()
	//                          ^ d: picker shop/pricing.go:27, shop/pricing.go:32
	//                                      ^ d: shop/pricing.go:27
	_ = t.Rate(); for _, t := range cs { t.Rate() }
	//    ^ d: shop/pricing.go:27
	//                                     ^ d: picker shop/pricing.go:27, shop/pricing.go:32
	_ = t.Rate(); { t := &Coupon{}; _ = t }
	//    ^ d: shop/pricing.go:27
	_ = t.Rate() // t := &Coupon{}; t.Rate()
	//    ^ d: shop/pricing.go:27
	s := "t := &Coupon{}; t.Rate()"; _ = t.Rate(); _ = s
	//                                     ^ d: shop/pricing.go:27
	_ = t == Tariff{}; _ = t.Rate()
	//                       ^ d: shop/pricing.go:27
	_ = rebate{t: t.Rate()}
	//              ^ d: shop/pricing.go:27
	t.Base = 1; _ = t.Rate()
	//                ^ d: shop/pricing.go:27
}

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

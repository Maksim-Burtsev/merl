package shop

type Stylus struct{}

func (s Stylus) Blot() string { return "stylus" }

type Nib struct{}

func (n Nib) Blot() string { return "nib" }

func QuotedA(s Stylus) string {
	return "s.Blot()" + `s.Blot()`
	//        ^ d: none
	//                     ^ d: none
}

func QuotedB(s Stylus) string {
	return `
	s.Blot() \`
	//^ d: none
}

func QuotedC(s Stylus) string {
	return "a\\" + s.Blot() + string('"') + s.Blot() // "
	//               ^ d: shop/quoted.go:5
	//                                        ^ d: shop/quoted.go:5
}

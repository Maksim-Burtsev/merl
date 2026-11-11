package shop

type Platter struct{}
type Saucepan struct{}

func (p *Platter) Dollop()  {}
func (s *Saucepan) Dollop() {}

func servePair() (*Platter, *Saucepan)            { return &Platter{}, &Saucepan{} }
func serveNamed() (plate *Platter, pan *Saucepan) { return }
func serveGrouped() (a, b *Saucepan)              { return }

func unpack() {
	plate, pan := servePair()
	pan.Dollop()
	//  ^ d: shop/servings.go:7
	plate.Dollop()
	//    ^ d: shop/servings.go:6
	_, second := serveNamed()
	second.Dollop()
	//     ^ d: shop/servings.go:7
	var near, far = servePair()
	far.Dollop()
	//  ^ d: shop/servings.go:7
	_ = near
	_, grouped := serveGrouped()
	grouped.Dollop()
	//      ^ d: !jump
	pans := map[string]*Saucepan{}
	got, ok := pans["x"]
	_ = ok
	got.Dollop()
	//  ^ d: picker shop/servings.go:6, shop/servings.go:7
}

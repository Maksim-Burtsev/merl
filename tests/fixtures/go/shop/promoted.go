package shop

type Rack struct{}

func (Rack) Stow() {}

type Aisle struct {
	Rack
}

type Bay struct{}

func (Bay) Stow() {}

type Dock struct {
	Aisle
	Bay
}

func Load(d Dock) {
	d.Stow()
	//^ d: shop/promoted.go:13
}

type Yard struct{}

func (Yard) Stow() {}

type Ramp struct {
	Bay
	Yard
}

func Unload(r Ramp) {
	r.Stow()
	//^ d: picker shop/promoted.go:5, shop/promoted.go:13, shop/promoted.go:27
}

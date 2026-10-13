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

type Closer interface {
	Close()
}

type ReadCloser interface {
	Closer
	Read()
}

type WriteCloser interface {
	Closer
	Write()
}

type Pipe interface {
	ReadCloser
	WriteCloser
}

func Shut(p Pipe) {
	p.Close()
	//^ d: shop/promoted.go:40
}

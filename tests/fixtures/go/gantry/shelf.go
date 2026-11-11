package gantry

type Shelf struct {
	Rail
	Gantry[Crate]
}

func (s *Shelf) Tidy() {
	s.First().Fill()
	//        ^ d: gantry/gantry.go:5
}

type Tall struct {
	*Shelf
}

type Deep struct {
	Gantry[*Crate]
}

type Bracket[U any] struct {
	Gantry[U]
}

func (b Bracket[U]) Tidy() {
	b.First().Fill()
	//        ^ d: picker gantry/gantry.go:5, gantry/gantry.go:9, …
}

func Use(t Tall, d Deep) {
	t.First().Fill()
	//        ^ d: gantry/gantry.go:5
	d.First().Fill()
	//        ^ d: gantry/gantry.go:5
	t.Wipe()
	//^ d: gantry/gantry.go:17
}

func NewShelf() *Shelf { return &Shelf{} }

func Made() {
	NewShelf().First().Fill()
	//                 ^ d: gantry/gantry.go:5
}

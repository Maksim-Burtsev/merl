package gantry

type Crate struct{}

func (c Crate) Fill() {}

type Barrel struct{}

func (b Barrel) Fill() {}

type Gantry[T any] struct{ load T }

func (g Gantry[T]) First() T { return g.load }

type Rail struct{}

func (r Rail) Wipe() {}

type Pair[K any, V any] struct{}

func (p Pair[V, K]) Left() V {
	var v V
	return v
}

type Twin struct {
	Pair[Crate, Barrel]
}

func Paired(tw Twin) {
	tw.Left().Fill()
	//         ^ d: gantry/gantry.go:5
}

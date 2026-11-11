package gantry

type Crate struct{}

func (c Crate) Fill() {}

type Barrel struct{}

func (b Barrel) Fill() {}

type Gantry[T any] struct{ load T }

func (g Gantry[T]) First() T { return g.load }

type Rail struct{}

func (r Rail) Wipe() {}

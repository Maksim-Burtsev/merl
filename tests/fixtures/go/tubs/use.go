package tubs

func each(xs []*Tub, f func(*Tub)) {}

func run(tubs []*Tub) {
	c := &Tun{}
	each(tubs, func(c *Tub) { c.Dunk() })
	//                          ^ d: tubs/tub.go:5
	c.Dunk()
	//^ d: tubs/tun.go:5
	each(tubs, func(x *Tub) { x.Dunk() })
	//                          ^ d: tubs/tub.go:5
	f := func(c *Tub) bool { return c.Dunk() }
	//                                ^ d: tubs/tub.go:5
	_ = f
	c.Dunk(); each(tubs, func(c *Tub) { c.Dunk() })
	//^ d: !jump
	each(tubs, func(c *Tub) {}); c.Dunk()
	//                             ^ d: !jump
	each(tubs, func(x *Tub) { c.Dunk() })
	//                          ^ d: tubs/tun.go:5
	for _, t := range tubs {
		t.Slop()
		//^ d: tubs/tub.go:6
	}
}

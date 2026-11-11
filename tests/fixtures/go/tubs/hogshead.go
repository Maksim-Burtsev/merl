package tubs

type Hogshead struct{}

func (t *Hogshead) Dunk() bool { return true }
func (t *Hogshead) Slop() int { return 1 }

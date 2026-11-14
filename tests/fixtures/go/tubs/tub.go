package tubs

type Tub struct{}

func (t *Tub) Dunk() bool { return true }
func (t *Tub) Slop() int { return 1 }

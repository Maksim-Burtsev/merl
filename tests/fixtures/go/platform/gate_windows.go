package platform

// Gate exists on two platforms and on no other: where neither is built, both are offered.
type Gate struct{}

func (g *Gate) Lift() int {
	return 1
}

func NewGate() *Gate {
	return &Gate{}
}

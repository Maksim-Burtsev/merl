package platform

type Gate struct{}

func (g *Gate) Lift() int {
	return 2
}

func NewGate() *Gate {
	return &Gate{}
}

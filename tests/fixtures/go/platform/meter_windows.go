package platform

type Meter struct{}

func (m *Meter) Sample() int {
	return 2
}

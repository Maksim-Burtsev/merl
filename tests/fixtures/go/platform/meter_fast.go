//go:build fast && !windows

package platform

// Meter: a tag that is no platform is not known to be built, so it is preferred to nothing.
type Meter struct{}

func (m *Meter) Sample() int {
	return 1
}

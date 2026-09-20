// +build !windows

package platform

// Meter: the old constraint is not read, so the file is not known to be built and wins nothing.
type Meter struct{}

func (m *Meter) Sample() int {
	return 1
}

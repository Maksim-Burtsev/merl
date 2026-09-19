//go:build !windows

package platform

type Gauge struct{}

func (g *Gauge) Read() int {
	return 2
}

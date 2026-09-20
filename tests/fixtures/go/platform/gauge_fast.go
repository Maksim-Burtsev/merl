//go:build windows && !slow

package platform

// Gauge: a platform and a tag of the project's own in one line.
type Gauge struct{}

func (g *Gauge) Read() int {
	return 1
}

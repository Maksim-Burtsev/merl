//go:build fast

package platform

// Codec is declared twice under a tag of the project's own, which no host decides.
type Codec struct{}

func (c *Codec) Encode() int {
	return 1
}

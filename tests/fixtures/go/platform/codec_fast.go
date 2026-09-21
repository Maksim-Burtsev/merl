//go:build fast

package platform

// Codec is declared twice under a tag of the project's own, unset until `-tags` names it.
type Codec struct{}

func (c *Codec) Encode() int {
	return 1
}

func (c *Codec) Twice() int {
	return c.Encode() * 2
}

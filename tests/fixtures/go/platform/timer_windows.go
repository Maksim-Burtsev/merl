package platform

func (t *Timer) Tick() int {
	return 1
}

// Inside the file the host does not build, neither Tick is preferred.
func (t *Timer) Twice() int {
	return t.Tick() * 2
}
